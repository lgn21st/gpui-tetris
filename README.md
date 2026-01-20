# gpui-tetris

A desktop Tetris built in Rust with `gpui`, targeting a fixed-size window on macOS (MacBook Air M2 tested). The UI is a simple two-panel layout: a 10x20 playfield and a side panel for next/hold/score.

## Features
- Resizable window with proportional scaling (base 480x720).
- Classic Tetris rules and scoring.
- Keyboard controls (arrow keys + space).
- Optional stereo SFX playback via `assets/sfx/*.wav`.

## Controls
- Left/Right: move piece
- Down: soft drop
- Up: rotate clockwise
- Space: hard drop
- C: hold
- Enter: start (title screen)
- P: pause/resume
- S: settings
- M: mute/unmute SFX
- +/-: adjust SFX volume
- 0: reset settings
- Cmd+Ctrl+F: toggle fullscreen

## Project Structure
- `src/main.rs`: app entry point.
- `src/ui/`: gpui window setup and rendering.
- `src/game/`: board, pieces, state, rules.
- `tests/`: unit tests for board and rules.
- `assets/` and `docs/`: optional resources and notes.

## Development
Requirements: Rust 2021 edition and `gpui = "0.2.2"` (vendored locally via `[patch.crates-io]`).

Common commands:
```bash
cargo run    # launch the app
cargo test   # run unit tests
cargo fmt    # format code
cargo clippy # lint
```

## Scoring (Classic)
Scores follow classic rules: 1/2/3/4 line clears award 40/100/300/1200 points, multiplied by (level + 1). Level increases every 10 lines. Soft drop awards 1 point per cell, hard drop awards 2 points per cell.

## Status
The UI renders the board with active and ghost pieces, inputs (including hold) are wired, auto-drop ticking runs each frame, and title/pause/game-over overlays plus next/hold previews are shown. Soft drop acceleration is input-driven with a short grace window, and left/right movement uses DAS/ARR repeat logic. Rotation uses SRS kick tables for I/J/L/S/T/Z pieces. Level speed follows a stepped curve and line clears briefly pause gravity with a flash effect; game over adds a red tint overlay and hides active/ghost pieces during line-clear pause. Lock delay resets are limited while grounded to prevent infinite stalling, with a pulsing lock-warning flash near expiry, a lock-delay bar in the HUD, and a landing spark highlight on lock. The default ruleset is classic line scoring; modern combo/B2B/T-spin scoring is available by switching the ruleset. Paused state ignores gameplay inputs, and focus loss auto-pauses. Sound events are emitted and, if `assets/sfx/` contains matching WAVs, played through the cpal mixer. Basic settings (SFX volume/mute/reset) are available in-game.

SFX file names:
`move.wav`, `rotate.wav`, `soft_drop.wav`, `hard_drop.wav`, `hold.wav`,
`line_clear_1.wav`..`line_clear_4.wav`, `game_over.wav`.

See `docs/audio_assets.md` for the Kenney Interface Sounds (CC0) mapping and license.

## Roadmap
- Consider audio polish once core rules stabilize.

## Dependency Notes (TODO)
- TODO: This project vendors `gpui` and `zed-font-kit` under `vendor/` to resolve a `core-graphics` version mismatch on macOS.
- TODO: Upstream `gpui` depends on `zed-font-kit` pinned to `core-graphics 0.24`, while `core-text` requires `core-graphics 0.25`, causing `CGFont/CGContext` type mismatches at compile time.
- TODO: When upstream aligns these versions, remove `vendor/`, drop the `[patch.crates-io]` overrides in `Cargo.toml`, and return to the crates.io dependency.
See `docs/dependencies.md` for full details and removal steps.
