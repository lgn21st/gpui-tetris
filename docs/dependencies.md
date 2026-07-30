# Dependencies

`rust-toolchain.toml` is the toolchain source of truth, `Cargo.toml` owns direct
requirements, and `Cargo.lock` records the resolved graph. Do not duplicate
their versions in prose.

GPUI is consumed from crates.io with runtime shaders. The previous GPUI,
zed-font-kit, and core-graphics vendor copies were removed after an unvendored
macOS build passed all repository gates. Reintroduce a local patch only for a
reproduced upstream incompatibility, with its reason and removal condition.

After dependency changes, run the gates in `AGENTS.md`, `cargo update
--dry-run`, and `cargo audit --no-fetch --no-yanked`. Known upstream ownership
includes a Linux-XCB-only XML parser advisory that is not reachable on macOS,
plus GPUI transitive future-incompatibility warnings. Recheck command output for
the current packages and versions.
