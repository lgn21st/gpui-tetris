# Architecture

## Boundaries

- `src/game/` owns deterministic rules and state without GPUI dependencies.
- `src/ui/` owns lifecycle, device input, rendering, and UI caches.
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

`TetrisView::render` drives the application loop:

1. Poll Adapter and controller input.
2. Apply accepted actions and advance bounded fixed steps.
3. Refresh changed render caches.
4. Publish the latest authoritative observation and render.
5. Request the next frame.

The Adapter has no worker thread. Sockets are nonblocking and observations are
coalesced, so slow clients cannot stall rendering. Exact transport limits and
startup policy are in `adapter-implementation-profile.md`.

`GameState` owns gameplay, timers, RNG, protocol identities, and events. Tick
order is defined in `rules-spec.md`; long frame stalls are truncated at the UI
boundary. Focus loss clears held UI input, while menus and device state remain
UI concerns.

## Change ownership

| Concern | Location |
| --- | --- |
| Movement, timing, rotation, scoring | `game/state/` |
| Pieces and board | `game/pieces.rs`, `game/board.rs` |
| HUD and game rendering | `ui/ui_state.rs`, `ui/render/` |
| Settings components and control events | `ui/view/settings.rs` |
| Protocol model, planning, transport | `adapter/` |
| Audio | `audio.rs` |
