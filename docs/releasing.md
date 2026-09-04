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

## Signing and notarization

The default is an ad-hoc signature, not Apple notarization. For Developer ID
distribution, install your certificate in Keychain and store notarization
credentials using `xcrun notarytool store-credentials`. Do not put passwords,
private keys or credentials in source, command files or release notes.

```bash
python3 -B scripts/release.py prepare \
  --verifier /absolute/path/to/adapter_verify.py \
  --identity 'Developer ID Application: YOUR NAME (TEAMID)' \
  --notary-profile YOUR_KEYCHAIN_PROFILE
```

The identity and profile must be supplied together. This path enables hardened
runtime and timestamps, waits for Apple's Accepted result, retains the notary
log, staples and validates the ticket, then recreates the ZIP. It verifies the
stapled app again after extraction. Actual notarization requires a valid paid
Apple Developer membership and credentials; having this code is not evidence
that a build has been notarized.

Apple's references: [notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution),
[customizing the workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).

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
