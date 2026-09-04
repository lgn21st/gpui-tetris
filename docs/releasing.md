# Release procedure

1. Update the package version in `Cargo.toml` and its local package entry in
   `Cargo.lock`; keep dependency versions locked.
2. Run the gates in `AGENTS.md` and the headless test configuration. Commit the
   exact source being released, push it, and inspect the corresponding CI run.
3. Run `cargo bundle --release`. Inspect the bundle version, executable
   architecture, icon and SFX resources. Seal the complete app with
   `codesign --force --deep --sign - target/release/bundle/osx/gpui-tetris.app`
   and verify with `codesign --verify --deep --strict`.
4. Archive with macOS `ditto -c -k --sequesterRsrc --keepParent`. Extract the ZIP
   outside the source tree and verify the signature and resources again.
   Launch that extracted app and run the upstream Adapter verifier against it.
5. Generate SHA-256 checksums and a build manifest containing the source commit,
   target architecture, toolchain and signing status. Keep binaries under the
   ignored `target/` directory.
6. Create an annotated version tag at the tested commit. Upload the ZIP,
   manifest and checksum file to a draft GitHub Release; verify the uploaded
   names, sizes and digests, then publish the release.

Current downloadable binaries target macOS Apple Silicon. They use an ad-hoc
signature and are not Apple-notarized. Release notes must state this, describe
observable compatibility changes, and distinguish automated checks from any
remaining manual device or visual validation. Do not describe a local signature
check as Developer ID signing or notarization.
