# Architecture

## Boundaries

- `src/game/` owns deterministic rules and state without GPUI dependencies.
- `src/runtime.rs` owns the authoritative game, lifecycle, clock and Adapter pump.
- `src/ui/` owns device input, presentation state, rendering, and UI caches.
- `src/adapter/` maps protocol data, plans placement through core rules, and
  runs the nonblocking TCP transport.
- `src/audio.rs` consumes bounded sound events; failure is non-fatal.

`GameAction` is the mutation boundary. UI and Adapter inputs both pass through
it, and Adapter placement reuses core collision, rotation, and scoring logic.

GPUI Kit is the only direct UI dependency. Its platform factory creates the
application, its asset provider embeds component icons, and its initializer
sets up the enabled layers before views are created. Window creation runs on
the application executor, following Kit's asynchronous startup pattern.

The window root is a `gpui_kit::component::Root` around `TetrisView`. Kit
supplies reusable settings controls; `TetrisView` retains game focus and
keyboard shortcuts. The board, HUD, overlays tied to gameplay, and animation
remain project-owned elements using `gpui_kit` rendering primitives so the
per-frame path does not depend on general-purpose widgets.

## Runtime

`TetrisView` owns a cancellable executor task that pumps `Runtime` every 8 ms,
independently of display callbacks. Its window context explicitly requests a
refresh after pumping; simulation never waits for that refresh. Hiding, occluding or minimizing the window
does not stop TCP or game time. `render` reads the game through `Runtime::state`
and updates presentation caches; it does not run game or network work.

Each pump applies accepted Adapter commands, advances bounded fixed steps with
local repeat actions, and publishes a full snapshot. The clock accumulates
`Duration`, retains fractional milliseconds, and caps catch-up at 15 steps.
The core pause state is authoritative. Opening settings requests a normal pause;
remote resume/restart closes that overlay instead of leaving a second tick gate.

Window deactivation clears held local input without pausing the core. Controller
polling has a per-pump event budget; inactive input and events queued before
reactivation are discarded. Pure repeat/mapping tests do not initialize devices.

`GameState` owns gameplay, timers, RNG and the latest transition's events.
Advancing logical_step replaces the event scope. Observations borrow the step
and events together, so throttling/coalescing skips whole transitions. Board
occupancy has one representation (`Cell.kind`), and its private revision changes
only when locked cells change. Runtime exposes no mutable game reference to UI.
The core's public construction APIs remain available for deterministic fixtures.

TCP remains on the application thread, with nonblocking sockets and separate
budgets for accepts, reads, frames, commands and writes. A single outbound cursor
protects started frames; response priority applies only at frame boundaries.
Optional diagnostic logging uses a bounded worker queue and rolling files.
Exact limits and startup policy are in `adapter-implementation-profile.md`.

HUD labels use shared strings and compare underlying values before formatting.
Board serialization uses a fixed cell array. Audio mixing owns its preallocated
voice buffer inside the callback, uses a bounded event drain, and converts
samples directly without locks or format-specific scratch buffers.

## Change ownership

| Concern | Location |
| --- | --- |
| Authoritative lifecycle and fixed clock | `runtime.rs` |
| Movement, timing, rotation, scoring | `game/state/` |
| Pieces and board | `game/pieces.rs`, `game/board.rs` |
| HUD and game rendering | `ui/ui_state.rs`, `ui/render/` |
| Settings components and control events | `ui/view/settings.rs` |
| Protocol model, planning, transport | `adapter/` |
| Audio | `audio.rs` |
