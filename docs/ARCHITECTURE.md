# Architecture

## Boundaries

- `src/game/` owns deterministic rules and state. It has no GPUI dependency.
- `src/ui/` owns GPUI lifecycle, raw input, render caches, and drawing.
- `src/adapter/mod.rs` owns protocol mapping, snapshots, and hashing;
  `planning.rs` searches place commands using core rules; `transport.rs` owns
  nonblocking TCP, buffering, and logging.
- `src/audio.rs` consumes bounded `SoundEvent` traffic; audio failure is
  non-fatal.

`GameAction` is the mutation boundary. Adapter placement reuses the core board
collision and SRS kick logic rather than implementing separate rules.

## Frame pipeline

`TetrisView::render` drives a single-threaded application loop:

1. Poll nonblocking Adapter and controller input.
2. Apply accepted actions through `UiState` to `GameState`.
3. Advance fixed 16 ms game steps; cap catch-up at 15 steps per frame.
4. Refresh board, preview, and label caches when their source revisions change.
5. Emit an authoritative Adapter observation and render the UI.
6. Request the next frame.

The Adapter has no worker thread. Its sockets are nonblocking and outbound
observations are coalesced so a slow client cannot stall rendering.

## State and timing

`GameState` owns the board, active/hold/next pieces, scoring, lifecycle flags,
timers, RNG, protocol identities, and bounded events. The top-level states are:

| State | UI `started` | Core `paused` | Core `game_over` |
| --- | --- | --- | --- |
| Title | false | false | false |
| Playing | true | false | false |
| Paused | true | true | false |
| Game over | true | false | true |

Each tick updates landing flash, line-clear pause, gravity, and lock delay in
that order. A line-clear pause consumes only its share of elapsed time. Long
frame stalls are truncated at the UI boundary to preserve responsiveness.

## Rendering and input

- `ui/view/events.rs`: GPUI events and focus handling.
- `ui/input.rs`: keyboard/controller holds and bounded DAS/ARR repeats.
- `ui/ui_state.rs`: session/UI state and render caches.
- `ui/render/`: layout, board, panel, overlays, and theme.

The core and audio event buffers hold at most 256 sound events; the mixer keeps
at most 16 active voices. Missing assets or output devices do not stop gameplay.

## Change map

| Concern | Primary location |
| --- | --- |
| Movement/locking | `game/state/actions.rs`, `game/state/timing.rs` |
| Rotation | `game/state/kicks.rs` |
| Scoring | `game/state/scoring.rs` |
| Piece shapes | `game/pieces.rs` |
| HUD/rendering | `ui/ui_state.rs`, `ui/render/` |
| Protocol/snapshots | `adapter/mod.rs` |
| Place planning | `adapter/planning.rs` |
| TCP/logging | `adapter/transport.rs` |
| Audio | `audio.rs` |

Behavioral contracts are in `docs/rules-spec.md`; Adapter-specific capacities
and policies are in `docs/adapter-implementation-profile.md`.
