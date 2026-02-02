# gpui-tetris Architecture (Beginner Friendly)

This document explains how the game is organized and how a single frame of
gameplay flows through the code. It is written for new programmers who want to
understand the control flow and where to look when changing behavior.

## Big Picture
Think of the app as two layers:
- **Game rules** (`src/game/`): pure logic, no UI code. This is where the rules
  for movement, rotation, gravity, line clears, scoring, and timing live.
- **UI and input** (`src/ui/`): gpui view code, keyboard/controller handling,
  and drawing. This layer reads state from the game rules and displays it.

Keeping these separate makes the rules easy to test and easier to reason about.

## Project Map
- `src/main.rs`: program entry. Creates the window and calls `ui::run()`.
- `src/game/`: game rules and data types.
  - `src/game/state/`: smaller files for actions, scoring, timing, kicks, rng.
  - `src/game/input.rs`: `GameAction` enum that represents inputs.
- `src/ui/`: gpui view, rendering helpers, HUD, and input repeat logic.
  - `src/ui/app.rs`: app bootstrap, menus, key bindings.
  - `src/ui/view.rs`: the main view; owns the render loop.
  - `src/ui/view/events.rs`: key/mouse event handlers.
  - `src/ui/input.rs`: keyboard/controller repeat handling (DAS/ARR).
  - `src/ui/ui_state.rs`: UI state and cached data for rendering.
  - `src/ui/render/`: board/panel/overlay drawing helpers and theme palette.
- `tests/`: unit tests for the rules.

## Core Data Types (Mental Model)
- **`GameAction`**: a small enum such as `MoveLeft`, `RotateCw`, `HardDrop`.
  UI code creates actions and sends them to the game rules.
- **`GameState`**: the main game data structure. Holds the board, active piece,
  next queue, hold piece, score, timers, and flags (paused/game over).
- **`UiState`**: wraps `GameState` plus UI-only data like HUD labels and caches.

If you are lost, search for where a `GameAction` is created and follow it.

## Board Coordinates (Simple Explanation)
The board is a 10x20 grid. Internally, the board is stored as a flat array
indexed row by row:
- Row 0 is the top, row 19 is the bottom.
- Column 0 is the left, column 9 is the right.
- A cell is "empty" or holds a tetromino type.

Rendering helpers convert this array into colored squares on screen.

## Game Loop: What Happens Each Frame
gpui calls the view's `render` method every animation frame. The flow is:
1. **Poll input**: collect controller events and buffered keyboard events.
2. **Apply actions**: map events to `GameAction` and forward to `UiState`.
3. **Tick the game**: advance time-based logic by `elapsed_ms`.
4. **Update UI caches**: refresh board/preview caches if the game changed.
5. **Render**: draw the board, overlays, and side panel.
6. **Request next frame**: keep the loop running.

This means "render" also drives the game logic. There is no separate thread.

## Input Flow (Keyboard and Controller)
Beginner-friendly path to trace:
1. gpui calls event handler in `src/ui/view/events.rs`.
2. The handler calls into `InputState` (in `src/ui/input.rs`).
3. `InputState` converts raw input to `GameAction`.
4. `UiState` receives `GameAction` and applies it to `GameState`.

### Repeat Movement (DAS/ARR)
Instead of moving every frame, the code uses a "repeat timer":
- **DAS**: delay before repeating starts.
- **ARR**: repeat speed once repeating starts.

`InputState` tracks which directions are held and emits repeated actions at the
right times.

## Timing Concepts (Gravity, Lock Delay, Line Clear Pause)
These timers live in the game rules:
- **Gravity**: how fast the piece falls automatically (depends on level).
- **Lock delay**: after a piece lands, it does not lock immediately. You can
  still rotate or move it. Each move resets the lock timer (up to a cap).
- **Line clear pause**: short pause after clearing lines to show a flash effect.

When the game ticks, it updates these timers using `elapsed_ms`.

## Piece Lifecycle (From Spawn to Lock)
1. **Spawn**: a new piece appears at the top.
2. **Fall**: gravity pulls it down each tick.
3. **Move/Rotate**: player actions move or rotate the piece.
4. **Touch down**: when it hits the stack, lock delay starts.
5. **Lock**: piece becomes part of the board.
6. **Clear**: if a full line exists, clear it and update score.
7. **Next**: spawn another piece, repeat.

Understanding this flow is the key to making game rule changes.

## Hold and Next Queue
- **Hold**: lets the player store a piece. There is usually a "once per piece"
  rule. `GameState` enforces it.
- **Next queue**: a small list of upcoming pieces shown in the panel.

Both are kept in `GameState` and rendered by `src/ui/render/panel.rs`.

## Rendering Breakdown
Rendering is split into helpers to keep logic clear:
- `src/ui/render/board.rs`: draws the board grid and active/ghost pieces.
- `src/ui/render/overlay.rs`: draws flashes, pause/game over tint, lock warnings.
- `src/ui/render/panel.rs`: draws score, level, next, hold, and status labels.
- `src/ui/render/theme.rs`: color definitions.

### UI Caches (Why They Exist)
Rendering the board every frame is expensive if nothing changed. `UiState`
maintains simple caches (board cells and preview masks). If the board revision
does not change, the cached data is reused to reduce work.

## Audio
The game rules emit sound events (move, rotate, drop, line clear, etc.). The UI
layer pulls these events and plays them through `AudioEngine`.

Audio is optional. If `assets/sfx/` is missing, the game still runs.

## Common "Where Do I Change X?" Guide
- **Movement rules**: `src/game/state/actions.rs` and `src/game/board.rs`.
- **Rotation behavior**: `src/game/state/kicks.rs`.
- **Scoring**: `src/game/state/scoring.rs`.
- **Level speed**: `src/game/state/timing.rs`.
- **HUD labels**: `src/ui/ui_state.rs` and `src/ui/render/panel.rs`.
- **Colors**: `src/ui/render/theme.rs`.

## How to Read the Code (Suggested Order)
1. `src/main.rs` and `src/ui/app.rs` (app boot).
2. `src/ui/view.rs` (render loop).
3. `src/ui/view/events.rs` (input handlers).
4. `src/ui/input.rs` (repeat logic).
5. `src/game/state/mod.rs` and `src/game/state/*` (core rules).
6. `src/ui/render/*` (drawing).

## Testing Notes
- Tests in `tests/` validate rules without UI.
- If you change game behavior, add a test there first.

## Build & Dev
- `cargo run` launches the app.
- `cargo test` runs unit tests.
- `cargo fmt` and `cargo clippy` keep code clean.
