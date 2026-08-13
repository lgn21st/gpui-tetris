# Dependencies

`rust-toolchain.toml` is the toolchain source of truth, `Cargo.toml` owns direct
requirements, and `Cargo.lock` records the resolved graph. Do not duplicate
their versions in prose.

GPUI is consumed from crates.io with runtime shaders. The previous GPUI,
zed-font-kit, and core-graphics vendor copies were removed after an unvendored
macOS build passed all repository gates. Reintroduce a local patch only for a
reproduced upstream incompatibility, with its reason and removal condition.

GPUI Component is pinned to a crates.io release compatible with the resolved
GPUI line. It supplies the window `Root` and reusable settings controls only;
custom game rendering remains directly on GPUI. Do not switch either dependency
to its Git main branch independently: both projects are pre-1.0 and their API
and type identities must resolve against the same GPUI source.

After dependency changes, run the gates in `AGENTS.md`, `cargo update
--dry-run`, and `cargo audit --no-fetch --no-yanked`. The resolved graph has no
reported vulnerabilities; current upstream warnings cover unmaintained
transitive crates and future Rust incompatibilities. Recheck command output for
the current packages and versions.
