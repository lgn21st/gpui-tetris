# Repository Guidelines

Goal: build a desktop Tetris game in Rust using `gpui`.

## Project Structure & Module Organization
- `src/`: Rust sources for the app.
  - `main.rs`: app entry and window setup.
  - `game/`: gameplay logic (board, pieces, rules).
  - `ui/`: gpui views, input handling, HUD.
- `assets/`: optional images/fonts/sfx.
- `docs/`: design notes and diagrams.
- `tests/`: unit tests for rules/board state.

## Build, Test, and Development Commands
- `cargo run` — run the desktop app.
- `cargo test` — run unit tests.
- `cargo fmt` — format Rust sources.
- `cargo clippy` — lint and catch common issues.

## Coding Style & Naming Conventions
- Use `rustfmt` defaults (4-space indentation).
- Module names: `snake_case` (e.g., `game_loop.rs`).
- Types: `PascalCase`; functions/vars: `snake_case`.
- Prefer small, pure functions in `game/` for testability.

## Testing Guidelines
- Use built-in Rust test framework.
- Focus tests on rules: rotation, collision, line clears, scoring.
- Name tests by behavior (e.g., `clears_single_line`).

## Commit & Pull Request Guidelines
No commit convention is visible yet. Until one is established:
- Use imperative, present-tense messages (e.g., “Add initial game loop”).
- Keep commits focused and small.
PRs should include:
- A short summary of changes and rationale.
- Steps to verify (commands and expected results).
- Screenshots or clips for UI changes if applicable.

## Technology Choices
- Language: Rust.
- UI: `gpui` for rendering, input, and window management.
- Game loop: fixed tick update (e.g., 60 Hz) with drop timer.
- State: immutable-ish core state in `game/`, UI reads state and dispatches actions.
- Window: fixed-size 480x720 (10x20 board with side panel on MacBook Air M2).

## Dependency Workarounds (TODO)
- TODO: `gpui` and `zed-font-kit` are vendored under `vendor/` to align `core-graphics` 0.25 on macOS.
- TODO: Remove vendored overrides and `[patch.crates-io]` once upstream dependency versions are fixed.

## Implementation Plan
1) Project setup: `Cargo.toml`, minimal `main.rs`, gpui window.
2) Core rules: board grid, tetromino definitions, rotation, collision checks.
3) Game loop: tick-based updates, soft/hard drop, lock delay.
4) Rendering: draw board, active piece, ghost, next/hold queue.
5) Scoring/levels: line clears, level speed curve.
6) UX polish: pause, restart, game over, input remap, assets/sfx (optional).
7) Tests: cover rule edge cases and regression scenarios.

## Current Milestones
- Inputs wired (arrows, space, `c`, `p`, `r`) with HUD status.
- Tick logic and lock delay implemented in `GameState`, driven by per-frame updates.
- Board renders active + ghost pieces with next/hold previews.
- Pause/game-over overlay rendered on the board.
- Soft drop uses a short input-driven grace window.
- Soft/hard drops award per-cell points; movement/rotation resets lock timer.
- DAS/ARR-style repeat movement is handled via key down/up with timers.
- Next queue is kept at a minimum size for previews.
- Level speed uses a stepped curve; line clears briefly pause gravity.
- Line clear flash and game-over tint are rendered in the board overlay.
- Default lock delay is 450ms; line clear pause is 180ms.
- Tests cover rules, tick, actions, hold, pause/restart, and rotation kicks.

## Agent-Specific Instructions
Keep this document aligned with the current gpui/Rust setup. Update commands and structure whenever new crates, scripts, or assets are added.
