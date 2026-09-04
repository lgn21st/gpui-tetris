# Release procedure

The release tool requires an Apple Silicon Mac, Python 3.11+, Rust, cargo-bundle,
cargo-audit, Xcode command-line tools, and GitHub CLI authentication for upload.
It reads the package version from Cargo.toml; dependency resolutions remain in
Cargo.lock. The existing published release is never overwritten.

## Prepare

Update the package version and lockfile entry for a new release, record changes
in a notes file, and commit the exact source. Obtain the upstream Adapter
verifier from the authoritative package described in AGENTS.md. Then run:

```bash
python3 -B scripts/release.py prepare --verifier /absolute/path/to/adapter_verify.py
```

Preparation requires a clean checkout. It fetches locked dependencies, runs
formatting, desktop/headless tests, Clippy, doc tests and the online vulnerability
and yanked-dependency gate. It builds the release app, verifies version,
architecture, icon and SFX hashes, signs the app, and creates a ZIP with ditto.
The ZIP is extracted outside the repository; its signature/resources and the
upstream protocol checks are verified against that extracted application. The
test owns and terminates its child app even if verification fails.

Completed candidates appear under `target/releases/v<VERSION>-<COMMIT>/`:

- macOS arm64 application ZIP;
- build-manifest.json with source/toolchain, archive and verifier hashes;
- dependency-audit.json with the fetched advisory database identity;
- SHA256SUMS covering each attachment.

An interrupted or failed preparation does not create a completed candidate.
Existing candidate directories are never replaced. Do not delete or edit files
in an already published candidate to reuse it for different source.

## Signing scope

Release bundles use an ad-hoc signature. Developer ID signing and Apple
notarization are explicitly outside the current project requirements; the tool
does not accept certificates or credentials. Notes and manifests must state
that the app is not notarized. Signature verification checks bundle integrity,
not Apple's approval to distribute it. First launch may require user approval
in macOS.

## Upload and publish

Push the source and an annotated version tag at that exact commit. Wait for CI
to pass, then run (replace the example paths/version with your candidate):

```bash
python3 -B scripts/release.py verify target/releases/vVERSION-COMMIT
python3 -B scripts/release.py upload target/releases/vVERSION-COMMIT \
  --notes /absolute/path/to/release-notes.md
```

Upload checks the candidate checksums, clean source identity, remote tag and CI
results before creating a draft. It downloads all uploaded assets and compares
them byte-for-byte. Add `--publish` to the upload command to publish automatically
after that verification. Without the flag the release remains a draft; inspect
it and publish through GitHub when ready. A failed upload/verification is left
as a draft for inspection, never silently retried with overwritten assets.

Notes must describe compatibility changes, actual signing status, and remaining
manual checks. A signature check does not mean Developer ID signing or
notarization. Automation verifies protocol/resource behavior, not physical
controller operation, speaker listening or every window occlusion scenario.
