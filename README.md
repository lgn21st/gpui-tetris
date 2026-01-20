# gpui-tetris

A desktop Tetris built in Rust with `gpui`, targeting macOS (MacBook Air M2 tested). The UI is a two-panel layout: a 10x20 playfield and a side panel for next/hold/score.

## Features
- Resizable window with proportional scaling (base 480x720).
- Classic Tetris rules and scoring.
- Keyboard controls (arrow keys + space).
- Xbox controller input on macOS (via Bluetooth).
- Title, settings, pause, and game-over overlays.
- Fullscreen toggle (Cmd+Ctrl+F).
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

### Xbox Controller (macOS, Bluetooth)
Default mapping:
- D-pad / Left Stick: move
- Down: soft drop
- A: rotate clockwise
- B: rotate counter-clockwise
- X: hold
- Y: hard drop
- Start: pause/resume
- Select/Back: restart

## Project Structure
- `src/main.rs`: app entry point.
- `src/ui/`: gpui window setup and rendering.
- `src/game/`: board, pieces, state, rules.
- `tests/`: unit tests for board and rules.
- `assets/` and `docs/`: optional resources and notes.

## Development
Requirements: Rust 2024 edition and `gpui = "0.2.2"` (vendored locally via `[patch.crates-io]`).

Common commands:
```bash
cargo run    # launch the app
cargo test   # run unit tests
cargo fmt    # format code
cargo clippy # lint
```

## macOS Packaging & Icon
This project is configured to include the app icon when bundling on macOS. The icon file is:

- `assets/icon.icns`

The bundling metadata is set in `Cargo.toml` under `[package.metadata.bundle]`.

To produce a macOS `.app` bundle, use `cargo-bundle`:
```bash
cargo install cargo-bundle
cargo bundle --release
```

## Scoring (Classic)
Scores follow classic rules: 1/2/3/4 line clears award 40/100/300/1200 points, multiplied by (level + 1). Level increases every 10 lines. Soft drop awards 1 point per cell, hard drop awards 2 points per cell.

## Status
- Board renders active + ghost pieces with next/hold previews and title/pause/game-over overlays.
- Inputs wired (move/rotate/drop/hold) with DAS/ARR; soft drop uses a short grace window.
- Rotation uses SRS kick tables for I/J/L/S/T/Z pieces.
- Classic line scoring is default; modern combo/B2B/T-spin scoring is available by switching rulesets.
- Lock delay resets are capped; HUD shows a lock-delay bar with a pulsing warning near expiry.
- Line clear pause + flash, landing spark highlight on lock, and game-over tint.
- Focus loss auto-pauses; in-game settings expose SFX volume/mute/reset.
- Sound events are emitted and played through the cpal mixer if `assets/sfx/` WAVs exist.

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
