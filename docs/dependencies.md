# Dependency Notes

## gpui / zed-font-kit core-graphics mismatch (TODO)
On macOS, `gpui` depends on `zed-font-kit`, which pins `core-graphics` to `0.24`. Meanwhile `core-text` requires `core-graphics` `0.25`, which causes `CGFont`/`CGContext` type mismatches at compile time.

### Current workaround
- Vendored `gpui` and `zed-font-kit` under `vendor/`.
- Bumped their `core-graphics` dependency to `0.25`.
- Patched in `Cargo.toml` via `[patch.crates-io]`.

### TODO (remove when fixed upstream)
- Delete `vendor/`.
- Remove `[patch.crates-io]` overrides.
- Revert to the crates.io `gpui` dependency.
