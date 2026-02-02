# Rules Spec

This document captures the game rules and timing constants used by this project.

## Core Constants
- Board: width=10, height=20
- Spawn position: (x=3, y=0)
- Next queue preview size: 3 (UI); internal queue is topped up to >=4 before each spawn.
- Tick step: 16 ms (default GameConfig.tick_ms)
- Base drop ms: 1000 (default GameConfig.base_drop_ms)
- Soft drop multiplier: 10
- Soft drop grace: 150 ms
- Lock delay: 450 ms
- Lock reset limit: 15
- Line clear pause: 180 ms
- Landing flash: 120 ms
- Input DAS: 150 ms (default)
- Input ARR: 50 ms (default)
- Soft drop ARR: 50 ms (default)

## Tetromino Shapes (Offsets)
Rotation order: North, East, South, West.

I:
- N: (0,1) (1,1) (2,1) (3,1)
- E: (2,0) (2,1) (2,2) (2,3)
- S: (0,2) (1,2) (2,2) (3,2)
- W: (1,0) (1,1) (1,2) (1,3)

O:
- All rotations: (1,0) (2,0) (1,1) (2,1)

T:
- N: (1,0) (0,1) (1,1) (2,1)
- E: (1,0) (1,1) (2,1) (1,2)
- S: (0,1) (1,1) (2,1) (1,2)
- W: (1,0) (0,1) (1,1) (1,2)

S:
- N: (1,0) (2,0) (0,1) (1,1)
- E: (1,0) (1,1) (2,1) (2,2)
- S: (1,1) (2,1) (0,2) (1,2)
- W: (0,0) (0,1) (1,1) (1,2)

Z:
- N: (0,0) (1,0) (1,1) (2,1)
- E: (2,0) (1,1) (2,1) (1,2)
- S: (0,1) (1,1) (1,2) (2,2)
- W: (1,0) (0,1) (1,1) (0,2)

J:
- N: (0,0) (0,1) (1,1) (2,1)
- E: (1,0) (2,0) (1,1) (1,2)
- S: (0,1) (1,1) (2,1) (2,2)
- W: (1,0) (1,1) (0,2) (1,2)

L:
- N: (2,0) (0,1) (1,1) (2,1)
- E: (1,0) (1,1) (1,2) (2,2)
- S: (0,1) (1,1) (2,1) (0,2)
- W: (0,0) (1,0) (1,1) (1,2)

## Rotation and Kicks (SRS)
- O: kick table is all zeros.
- I and JLSTZ: use standard SRS kick tables.
- Try kicks in order; first valid placement wins.

## RNG and Queue
- Queue refill uses repeated "2-piece chunks" from shuffled 7-piece bags.
- Each refill shuffles [I,O,T,S,Z,J,L], then appends the first 2 pieces only.
- LCG RNG (Numerical Recipes constants) for deterministic tests.
- Before spawning the next active piece, queue is refilled to length >=4, then one piece is drawn.
- Result: after each spawn, queue length is >=3 (matching the 3-piece preview).

## Actions
GameAction:
- MoveLeft, MoveRight
- SoftDrop, HardDrop
- RotateCw, RotateCcw
- Hold
- Pause, Restart

Rules:
- If game_over, only Restart is accepted.
- If paused, only Pause/Restart is accepted.

### Movement
- Move: attempt dx +/-1; on success update ghost; lock reset may trigger.
- SoftDrop: try move +1; if success score +1. Soft drop activates grace timer.
- HardDrop: move down until blocked; +2 per cell; lock piece immediately.
- Rotate: apply SRS kicks; set last_action_rotate based on success.

### Hold
- Only once per spawn (can_hold flag).
- First hold stores current kind and spawns next.
- Subsequent hold swaps active with held.

## Timing
### Drop Interval
Table by level index:
- 0..8: [1000, 800, 650, 500, 400, 320, 250, 200, 160]
- >=9: 120 ms
Clamp: interval = min(table, base_drop_ms) then max(100 ms).
Soft drop: interval /= soft_drop_multiplier, min 1 ms.

### Tick
If game_over or paused: no updates.
Order per tick:
1) Landing flash timer.
2) Line clear pause timer (if active, stop here).
3) Drop timer update.
4) Apply gravity steps while drop_timer >= interval.
5) Lock timer update.

### Lock Delay + Resets
- If can move down: lock_timer=0 and lock_reset_count=0.
- If grounded: lock_timer += elapsed.
- When lock_timer >= lock_delay: lock piece, reset timers.
- Lock reset on grounded movement/rotation until limit:
  - If grounded and lock_reset_count < lock_reset_limit: lock_timer=0; lock_reset_count++.

### Soft Drop Grace
- When soft drop activated, soft_drop_timeout_ms = soft_drop_grace_ms.
- Timer counts down even if no further soft drop input; when it hits 0, soft_drop_active=false.

## Line Clears and Scoring
### Classic (default)
- Line scores: 1/2/3/4 = 40/100/300/1200
- Score = line_score * (level+1)
- Level increases every 10 lines (level = lines / 10).

### Modern (optional)
- T-spin scoring (full/mini) uses RulesConfig tables.
- B2B applies if T-spin full with lines or Tetris; bonus multiplier 3/2.
- Combo: +combo_base * combo_index (combo starts at 0, increases each consecutive clear).

### T-Spin Detection
- Only if active piece is T and last action was a rotation.
- Check 4 corner cells around T center; if >=3 filled, T-spin.
- If both front corners filled -> Full, else Mini.

## State Machine

This project models gameplay state as a combination of one UI gate flag and two Core flags:
- UI gate: `started` (false means title screen; true means gameplay session exists).
- Core flags: `paused`, `game_over`.

Derived top-level states:

| State | started | paused | game_over | Notes |
| --- | --- | --- | --- | --- |
| Title | false | false | false | Start-gated; gameplay inputs are not applied yet. |
| Playing | true | false | false | Normal fixed-step update and action handling. |
| Paused | true | true | false | Tick early-returns; only Pause/Restart accepted. |
| GameOver | true | false | true | Tick early-returns; only Restart accepted. |

### State transitions

| From | Event | To | Rules |
| --- | --- | --- | --- |
| Title | Start key (`Enter`/`Return`/`Space`) | Playing | Initializes a new run via restart with current RNG stream. |
| Title | Any mapped input except start | Playing (or consumed) | Driver starts first, then applies input; `pause`/`restart` are consumed to avoid double-transition. |
| Playing | `Pause` action | Paused | Toggles pause on. Also clears soft-drop active state and timeout. |
| Paused | `Pause` action | Playing | Toggles pause off. |
| Playing | Spawn blocked on lock/spawn path | GameOver | Sets `game_over = true`; emits game-over sound event. |
| GameOver | `Restart` action | Playing | Resets board/state; preserves configured restart seed mode semantics. |
| Paused | `Restart` action | Playing | Resets board/state immediately. |
| Playing | Focus loss (app inactive) | Paused | Auto-pause only when started and not game over. |

### Input acceptance by state

| State | Accepted actions |
| --- | --- |
| Title | Start keys; other mapped gameplay inputs first trigger start. |
| Playing | All gameplay actions. |
| Paused | `Pause`, `Restart`. |
| GameOver | `Restart`. |

## Visual/UX-Driven State
- The state machine above controls Title/Pause/GameOver overlays.
- Line clear pause hides active/ghost (render rule).
- Ghost guidance hides while grounded, during lock delay, during line clear pause, and until the active piece has successfully moved since spawn.
- Landing flash uses last locked cell positions and fades by remaining timer ratio.

## Remote Control via Adapter (TCP)
- Remote clients can observe state and send control commands through the adapter transport.
- Transport/protocol details are documented in `docs/adapter-protocol.md`.
- Architecture-level networking defaults (host/port, controller/observer behavior) are documented in `docs/architecture.md`.
- Adapter observation fields mirror core gameplay state, including paused/game-over and board snapshot fields.

## Sound Events (optional)
- Move, Rotate, SoftDrop, HardDrop, Hold, LineClear(n), GameOver.
