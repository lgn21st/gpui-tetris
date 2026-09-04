# Dependencies

`rust-toolchain.toml` is the toolchain source of truth, `Cargo.toml` owns direct
requirements, and `Cargo.lock` records the resolved graph. Do not duplicate
their versions in prose.

[GPUI Kit](https://github.com/longbridge/gpui-kit) is the sole direct UI
framework dependency. All UI types and macros come from `gpui_kit`; styled
controls and the window `Root` come from `gpui_kit::component`. Kit selects the
matching `gpui-pre` platform family, and enables runtime shaders on macOS
through its platform dependency. Build commands therefore need no explicit
`gpui/runtime_shaders` feature.

Startup uses `gpui_kit::application()`, installs Kit's embedded icon assets,
and calls `gpui_kit::init` before constructing controls. Keep the framework
release pinned and update it as a unit; do not add independent direct GPUI or
GPUI Component dependencies. The game board uses Kit's exported rendering
primitives while deterministic rules remain independent of the UI framework.

The macOS graphics/font graph uses upstream crates, including `core-graphics`
and `zed-font-kit`. No local patches are required. Introduce a local patch
only for a reproduced upstream incompatibility, with a removal condition.

For UI dependency changes, inspect the resolved macOS graphics/font graph with
`cargo tree` and confirm that Kit still enables runtime shaders.

After dependency changes, run the gates in `AGENTS.md`, `cargo update
--dry-run`, and `cargo audit --no-fetch --no-yanked`. The resolved graph has no
reported vulnerabilities; current upstream warnings cover unmaintained
transitive crates and future Rust incompatibilities. Recheck command output for
the current packages and versions.

The maintenance warnings are owned by the upstream Kit/GPUI dependency graph:
`bincode` comes through Base's syntax-highlighting stack, `instant` through
Base/Component timing, and `paste` is a build-time macro used by Component and
the graphics stack. `rustls-pemfile` comes through GPUI's HTTP client; the game
does not initiate HTTP requests. `rustybuzz` and `ttf-parser` are in the SVG/font
rendering stack. Reassess these paths on Kit upgrades rather than patching
independent framework crates locally.
