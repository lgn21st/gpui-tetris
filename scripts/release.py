#!/usr/bin/env python3
"""Build, verify and publish a macOS release from an exact, clean commit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import socket
import subprocess
import sys
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
REPO = "lgn21st/gpui-tetris"


def run(*args, capture=False, cwd=ROOT, **kwargs):
    return subprocess.run(
        [str(arg) for arg in args], cwd=cwd, check=True, text=True,
        stdout=subprocess.PIPE if capture else None, **kwargs
    ).stdout


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source():
    if run("git", "status", "--porcelain", capture=True).strip():
        raise ValueError("Commit or remove pending source changes before preparing a release")
    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]
    return package, run("git", "rev-parse", "HEAD", capture=True).strip()


def verify_bundle(app, package):
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    bundle = package["metadata"]["bundle"]
    if info["CFBundleShortVersionString"] != package["version"]:
        raise ValueError("Bundle version does not match source")
    if info["CFBundleIdentifier"] != bundle["identifier"]:
        raise ValueError("Bundle identifier does not match source")
    binary = app / "Contents/MacOS" / info["CFBundleExecutable"]
    arch = run("lipo", "-archs", binary, capture=True).strip()
    if arch != "arm64":
        raise ValueError(f"Expected arm64, got {arch}")
    resources = app / "Contents/Resources"
    if digest(resources / info["CFBundleIconFile"]) != digest(ROOT / bundle["icon"][0]):
        raise ValueError("Bundle icon differs from source")
    expected = {p.name: digest(p) for p in (ROOT / "assets/sfx").glob("*.wav")}
    actual = {p.name: digest(p) for p in resources.rglob("*.wav")}
    if not expected or expected != actual:
        raise ValueError("Bundled SFX differ from source")
    run("codesign", "--verify", "--deep", "--strict", app)
    return binary, info


def verify_protocol(binary, verifier):
    # The child is owned by this check and always terminated, including on failure.
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = dict(os.environ, TETRIS_AI_HOST="127.0.0.1", TETRIS_AI_PORT=str(port))
    env.pop("TETRIS_AI_DISABLED", None)
    with tempfile.TemporaryDirectory(prefix="gpui-protocol-") as work:
        with open(Path(work) / "app.log", "w+") as log:
            child = subprocess.Popen([str(binary)], cwd=work, env=env, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                while True:
                    if child.poll() is not None or time.monotonic() > deadline:
                        log.seek(0)
                        raise RuntimeError("Release app did not start: " + log.read())
                    try:
                        with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                            break
                    except OSError:
                        time.sleep(0.1)
                run(sys.executable, verifier, "all", "--port", port, "--timeout", 4, "--pieces", 6)
            finally:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()


def zip_app(app, archive):
    run("ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, archive)


def prepare(args):
    package, commit = source()
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Prepare requires an Apple Silicon Mac")
    verifier = args.verifier.resolve(strict=True)
    out = ROOT / "target/releases" / f'v{package["version"]}-{commit[:12]}'
    if out.exists():
        raise ValueError(f"Refusing to replace an existing candidate: {out}")
    run("cargo", "fetch", "--locked")
    run("cargo", "fmt", "--all", "--", "--check")
    run("cargo", "test", "--all-targets", "--locked", "--offline")
    run("cargo", "test", "--all-targets", "--locked", "--offline", "--no-default-features")
    run("cargo", "clippy", "--all-targets", "--locked", "--offline", "--", "-D", "warnings")
    run("cargo", "clippy", "--all-targets", "--locked", "--offline", "--no-default-features", "--", "-D", "warnings")
    run("cargo", "test", "--doc", "--locked", "--offline")
    audit = run("cargo", "audit", "--deny", "yanked", "--json", capture=True)
    # cargo-bundle has no --locked flag; prebuild locked, then package offline.
    run("cargo", "build", "--release", "--locked", "--offline")
    run("cargo", "bundle", "--release", env=dict(os.environ, CARGO_NET_OFFLINE="true"))
    app = ROOT / "target/release/bundle/osx" / (package["metadata"]["bundle"]["name"] + ".app")
    run("codesign", "--force", "--sign", "-", app)
    verify_bundle(app, package)
    # Only complete candidates are moved into the final output directory.
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".candidate-", dir=out.parent) as staging:
        staged = Path(staging)
        archive = staged / f'gpui-tetris-v{package["version"]}-macos-arm64.zip'
        zip_app(app, archive)
        with tempfile.TemporaryDirectory(prefix="gpui-release-check-") as extracted:
            run("ditto", "-x", "-k", archive, extracted)
            binary, info = verify_bundle(Path(extracted) / app.name, package)
            verify_protocol(binary, verifier)
        if source()[1] != commit:
            raise ValueError("Source changed during preparation")
        manifest = {
            "version": package["version"], "source_commit": commit,
            "target": "aarch64-apple-darwin", "archive": archive.name,
            "archive_sha256": digest(archive), "bundle_version": info["CFBundleVersion"],
            "rustc": run("rustc", "-Vv", capture=True).strip(),
            "cargo_bundle": run("cargo", "bundle", "--version", capture=True).strip(),
            "signing": "ad-hoc",
            "apple_notarized": False,
            "verifier_sha256": digest(verifier),
            "validation": ["tests", "headless tests", "clippy", "doc tests", "format",
                           "online audit", "bundle resources", "signature", "adapter conformance"],
            "manual_validation": "See release notes; automation does not certify devices or visual behavior",
        }
        (staged / "build-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        (staged / "dependency-audit.json").write_text(audit)
        (staged / "SHA256SUMS").write_text("".join(
            f"{digest(p)}  {p.name}\n" for p in sorted(staged.iterdir())
        ))
        staged.rename(out)
    print(out)


def verify_candidate(directory):
    manifest = json.loads((directory / "build-manifest.json").read_text())
    files = []
    for line in (directory / "SHA256SUMS").read_text().splitlines():
        expected, name = line.split("  ", 1)
        if Path(name).name != name or name in {".", "..", "SHA256SUMS"}:
            raise ValueError("Invalid checksum filename")
        path = directory / name
        if path.is_symlink() or digest(path) != expected:
            raise ValueError(f"Checksum mismatch: {name}")
        files.append(path)
    required = {manifest["archive"], "build-manifest.json", "dependency-audit.json"}
    if not required.issubset({p.name for p in files}) or len(files) != len(set(files)):
        raise ValueError("Missing or duplicated release assets")
    if digest(directory / manifest["archive"]) != manifest["archive_sha256"]:
        raise ValueError("Archive does not match manifest")
    return manifest, files + [directory / "SHA256SUMS"]


def publish(args):
    package, commit = source()
    manifest, files = verify_candidate(args.directory.resolve())
    if (manifest["version"], manifest["source_commit"]) != (package["version"], commit):
        raise ValueError("Candidate does not match current committed source")
    tag = "v" + package["version"]
    remote = run("git", "ls-remote", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}", capture=True)
    if not any(line.split()[0] == commit for line in remote.splitlines()):
        raise ValueError("Push the release tag at the tested commit before publishing")
    runs = json.loads(run("gh", "run", "list", "--repo", REPO, "--workflow", "ci.yml",
                          "--commit", commit, "--limit", 20, "--json", "status,conclusion", capture=True))
    if not runs or any(r["status"] != "completed" or r["conclusion"] != "success" for r in runs):
        raise ValueError("All CI runs for this commit must have completed successfully")
    notes = args.notes.resolve(strict=True)
    # Existing releases are never edited or overwritten by this command.
    run("gh", "release", "create", tag, "--repo", REPO, "--verify-tag", "--draft",
        "--title", tag, "--notes-file", notes, *files)
    with tempfile.TemporaryDirectory(prefix="gpui-upload-check-") as downloaded:
        run("gh", "release", "download", tag, "--repo", REPO, "--dir", downloaded)
        for local in files:
            if digest(Path(downloaded) / local.name) != digest(local):
                raise ValueError("Uploaded asset differs; release remains a draft")
    if args.publish:
        run("gh", "release", "edit", tag, "--repo", REPO, "--draft=false", "--latest")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("prepare")
    build.add_argument("--verifier", type=Path, required=True, help="Upstream adapter_verify.py")
    build.set_defaults(action=prepare)
    check = commands.add_parser("verify")
    check.add_argument("directory", type=Path)
    check.set_defaults(action=lambda args: verify_candidate(args.directory))
    upload = commands.add_parser("upload")
    upload.add_argument("directory", type=Path)
    upload.add_argument("--notes", type=Path, required=True)
    upload.add_argument("--publish", action="store_true", help="Publish after uploaded files match")
    upload.set_defaults(action=publish)
    args = parser.parse_args()
    try:
        args.action(args)
    except (ValueError, RuntimeError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Release failed: {error}\n")


if __name__ == "__main__":
    main()
