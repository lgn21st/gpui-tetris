# Local Release builds

This internal project uses local Release builds. Build directly from the current
working tree, including uncommitted changes:

```bash
./scripts/release.sh
```

Prerequisites: macOS, the repository's Rust toolchain, Xcode command-line tools,
and cargo-bundle. Install cargo-bundle once if needed:

```bash
cargo install cargo-bundle --locked
```

The script compiles with the release profile and locked dependencies, packages
the application with its icon and SFX, and completes the local app bundle. It
requires no Git tag, clean checkout, GitHub credentials, remote CI result,
release notes or external Adapter verifier. Checks described in AGENTS.md remain
separate from the build command.

Output (updated on each build):

- `target/release/gpui-tetris`: optimized executable;
- `target/release/bundle/osx/gpui-tetris.app`: self-contained local application.

Open the application from Finder, or:

```bash
open target/release/bundle/osx/gpui-tetris.app
```

When dependencies are already cached, build entirely offline:

```bash
CARGO_NET_OFFLINE=true ./scripts/release.sh
```

Copy the `.app` outside the repository for packaging checks. The script does not
launch the app, create archives or publish/upload anything. Version requirements
remain in Cargo.toml, Cargo.lock and rust-toolchain.toml. Earlier audit documents
record historical publishing work; this document and AGENTS.md define the
current local-only scope.
