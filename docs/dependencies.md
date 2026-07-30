# Dependencies

- Rust 2024 edition with MSRV 1.97.1; `rust-toolchain.toml` follows stable and
  installs rustfmt and Clippy.
- GPUI 0.2.2 comes directly from crates.io with runtime shaders enabled.
- CPAL 0.18.1 provides audio and gilrs 0.11.2 provides controller input.

A 2026-07-30 crates.io check found every direct dependency current. A verbose
`cargo update --dry-run` found no compatible lockfile update; five newer
transitive releases remain constrained by upstream GPUI requirements.

The former local GPUI, zed-font-kit, and core-graphics copies were removed after
an unvendored macOS build resolved consistently to `core-text 21.0.0` and
`core-graphics 0.24.0` and passed the full test and Clippy gates. Do not reintroduce
vendor code without a reproduced upstream incompatibility and a documented
removal condition.

After dependency changes run the gates in `AGENTS.md` and
`cargo audit --no-fetch --no-yanked`.

Current upstream-only risks:

- Linux XCB retains `quick-xml 0.30.0`, producing two RustSec findings not
  reachable on macOS; Wayland uses fixed 0.41.0.
- RustSec reports ten informational transitive warnings.
- GPUI still brings future-incompatible `block 0.1.6` and
  `proc-macro-error2 2.0.1`.
