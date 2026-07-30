# Rules Contract

This document records behavior that callers, UI, Adapter, and tests rely on.
Piece offsets themselves live only in `src/game/pieces.rs`.

## Constants

| Rule | Value |
| --- | --- |
| Board / spawn | 10×20 / `(3, 0)` |
| Fixed step | 16 ms |
| Gravity | 1000 ms base, 100 ms floor |
| Soft drop | 10× gravity, 150 ms grace |
| Lock | 450 ms delay, 15 resets |
| Line-clear pause / landing flash | 180 / 120 ms |
| Input DAS / ARR / soft-drop ARR | 150 / 50 / 50 ms |
| Next pieces | UI 3, protocol 5, post-spawn queue ≥5 |

## Pieces, rotation, and queue

- Rotations cycle North, East, South, West.
- I and JLSTZ use standard SRS kicks converted to positive-down board Y; O uses
  zero offsets. The first valid kick wins.
- A deterministic LCG shuffles all seven tetrominoes, then appends the first two
  per refill. Before spawn the queue is filled to at least six, then one piece
  is consumed. This is intentionally not advertised as a seven-bag.
- Hold is available once per spawned piece. First hold stores; later holds swap
  and respawn at the standard origin.

## Actions and lifecycle

- Movement and rotation apply only when `Board::can_place` succeeds.
- Soft drop awards one point per moved cell; hard drop awards two and locks
  immediately.
- Successful horizontal movement or rotation may reset grounded lock delay,
  up to the per-piece limit. Becoming airborne clears the timer but does not
  restore consumed resets.
- Failed move/rotate/drop attempts do not emit success sounds.
- Paused games accept only pause/restart; game-over accepts only restart.
- Restart resets gameplay with the selected seed and advances episode,
  logical-step, and board identities.

## Tick order

1. Decrease landing flash.
2. Consume line-clear pause; forward only leftover elapsed time.
3. Decrease soft-drop grace and choose the active drop interval.
4. Apply gravity; stop after the first blocked attempt.
5. Advance grounded lock delay and lock at the threshold.

Level gravity intervals are `1000, 800, 650, 500, 400, 320, 250, 200, 160`,
then 120 ms, clamped by configured base and the 100 ms floor.

## Scoring

Classic line clears award `40/100/300/1200 × (level + 1)` for one through four
lines; level is `lines / 10`. Modern mode adds configured T-spin, combo, and
B2B scoring. A difficult clear continues B2B, an ordinary clear breaks it, and
a no-line placement resets combo while preserving B2B. Arithmetic saturates;
custom B2B denominators are floored at one.

A T-spin requires a T piece, a successful last rotation, and at least three
occupied pivot corners. Both front corners produce Full; otherwise it is Mini.

## UI-derived behavior

- Line-clear pause hides active and ghost pieces.
- Ghost guidance also hides while grounded/locking or before the active piece
  first moves.
- Focus loss pauses a started local game, except when Adapter mode is enabled.
- Landing flash renders the last locked cells until its timer expires.

Transport semantics are maintained in `docs/adapter_acceptance.md`; local
runtime choices are in `docs/adapter-implementation-profile.md`.
