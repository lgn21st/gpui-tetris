# gpui-tetris

A desktop Tetris built in Rust with `gpui`, targeting a fixed-size window on macOS (MacBook Air M2 tested). The UI is a simple two-panel layout: a 10x20 playfield and a side panel for next/hold/score.

## Features
- Fixed window size: 480x720.
- Classic Tetris rules and scoring.
- Keyboard controls (arrow keys + space).

## Controls
- Left/Right: move piece
- Down: soft drop
- Up: rotate clockwise
- Space: hard drop
- (Planned) Shift/Ctrl: hold

## Project Structure
- `src/main.rs`: app entry point.
- `src/ui/`: gpui window setup and rendering.
- `src/game/`: board, pieces, state, rules.
- `tests/`: unit tests for board and rules.
- `assets/` and `docs/`: optional resources and notes.

## Development
Requirements: Rust 2021 edition and `gpui = "0.12"`.

Common commands:
```bash
cargo run    # launch the app
cargo test   # run unit tests
cargo fmt    # format code
cargo clippy # lint
```

## Scoring (Classic)
Scores follow classic rules: 1/2/3/4 line clears award 40/100/300/1200 points, multiplied by (level + 1). Level increases every 10 lines.

## Status
The rendering scaffold is in place; the next milestones are game loop timing, input mapping, and rule enforcement (movement, rotation, lock, and line clears).

## Roadmap
- Wire keyboard input (arrow keys + space) to `GameAction`.
- Implement tick-based game loop with drop timer and lock delay.
- Add piece movement, rotation with collision checks, and line clears.
- Render active/ghost pieces and next/hold panels.
- Add scoring/leveling UI and game over flow.

## Dependency Notes (TODO)
- TODO: This project vendors `gpui` and `zed-font-kit` under `vendor/` to resolve a `core-graphics` version mismatch on macOS.
- TODO: Upstream `gpui` depends on `zed-font-kit` pinned to `core-graphics 0.24`, while `core-text` requires `core-graphics 0.25`, causing `CGFont/CGContext` type mismatches at compile time.
- TODO: When upstream aligns these versions, remove `vendor/`, drop the `[patch.crates-io]` overrides in `Cargo.toml`, and return to the crates.io dependency.
See `docs/dependencies.md` for full details and removal steps.
