# Repository Guidelines

Build a desktop Tetris in Rust with GPUI. Toolchain and dependency versions
live only in `rust-toolchain.toml`, `Cargo.toml`, and `Cargo.lock`.

## Ownership

- `src/game/`: deterministic board, pieces, actions, scoring, timing, RNG.
- `src/ui/`: GPUI lifecycle, device input, HUD, rendering, and UI caches.
- `src/adapter/mod.rs`: Adapter protocol model, mapping, and snapshots.
- `src/adapter/planning.rs`: place-command search over core rules.
- `src/adapter/transport.rs`: nonblocking TCP lifecycle, buffers, and logging.
- `src/audio.rs`: optional bounded CPAL mixer and WAV loading.
- `tests/`: behavior and regression coverage.
- `docs/`: maintained architecture, rules, protocol evidence, and operations.
- `.github/`: CI and weekly dependency-update policy.

Keep game logic independent of GPUI. Route input through `GameAction`; reuse
core collision/SRS/scoring from Adapter planning. Bound queues, buffers, event
collections, and catch-up loops.

## Workflow

1. Inspect `git status` and preserve unrelated changes.
2. Add or update a focused test before behavior changes when practical.
3. Change the owning module; avoid parallel implementations.
4. Update the relevant maintained document when behavior, structure,
   dependencies, commands, or operational choices change.
5. Run focused tests, then the full gates.

```bash
cargo test --all-targets --offline --features gpui/runtime_shaders
cargo clippy --all-targets --offline --features gpui/runtime_shaders -- -D warnings
cargo test --doc --offline --features gpui/runtime_shaders
cargo fmt --all -- --check
git diff --check
```

Use rustfmt defaults and behavior-based test names. Use imperative commit
messages and keep commits focused. UI changes should include a visual check;
Adapter TCP tests may require permission to bind loopback sockets.

For release checks, run `cargo bundle --release`, launch the generated app
outside the source tree, and verify icon, bundled SFX, input, and resizing.

## Adapter protocol

The authority is the
[`lgn21st/tui-tetris` Adapter package](https://github.com/lgn21st/tui-tetris/tree/main/protocol/adapter).
Before Adapter work, read it in this order:

1. [`VERSION`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/VERSION)
2. [`CHANGELOG.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/CHANGELOG.md)
3. [`SPEC.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/SPEC.md)
4. [`schema.json`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/schema.json)
5. [`profiles/tcp-json-lines.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/profiles/tcp-json-lines.md)

Maintain `docs/adapter_acceptance.md` as the requirement-to-test matrix and
keep project-specific runtime choices in
`docs/adapter-implementation-profile.md`. Do not copy the normative protocol.
