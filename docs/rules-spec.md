# Rules Contract

Only behavior shared by the core, UI, Adapter, and tests belongs here. Piece
offsets live in `src/game/pieces.rs`.

| Rule | Value |
| --- | --- |
| Board / spawn | 10×20 / `(3, 0)` |
| Fixed step | 16 ms |
| Gravity | 1000 ms base, 100 ms floor |
| Soft drop | 10× gravity, 150 ms grace |
| Lock | 450 ms delay, 15 resets |
| Clear pause / landing flash | 180 / 120 ms |
| DAS / ARR / soft-drop ARR | 150 / 50 / 50 ms |
| Next pieces | UI 3, protocol 5, post-spawn queue ≥5 |

## Pieces and actions

- Rotation cycles North/East/South/West. I and JLSTZ use SRS kicks with
  positive-down Y; O does not kick. The first valid position wins.
- The deterministic LCG shuffles seven pieces and appends the first two on each
  refill; this is not a seven-bag. Hold is allowed once per spawn.
- Actions apply only at valid positions. Soft drop awards one point per cell;
  hard drop awards two and locks immediately.
- Successful grounded movement or rotation resets lock delay up to its limit.
  Becoming airborne clears both the timer and consumed-reset count, restoring
  the reset budget for the next grounded phase.
- Failed actions emit no success sound. Pause and game-over restrict accepted
  actions to their lifecycle controls. Restart resets from the selected seed
  and advances protocol identities.

## Timing

Each tick decreases landing flash and consumes clear pause. If the pause
expires, the full current fixed step resumes soft-drop grace, all gravity moves
covered by the accumulated interval, and grounded lock delay.

Level gravity intervals are `1000, 800, 650, 500, 400, 320, 250, 200, 160`,
then 120 ms, clamped by the configured base and floor.

## Scoring

Classic clears award `40/100/300/1200 × (level + 1)`; level is `lines / 10`.
Modern mode adds configured T-spin, combo, and B2B scoring. Difficult clears
continue B2B; ordinary clears and no-line placements break it. Base and B2B
points receive the level multiplier before the unscaled combo bonus is added.
Arithmetic saturates and custom denominators are never zero.

A T-spin requires a T piece, a successful last rotation, and three occupied
pivot corners. Both front corners make it Full; otherwise it is Mini.

## UI effects

Clear pause hides active and ghost pieces. Ghost guidance also hides while
grounded or before the piece first moves. Focus loss clears held device input
without changing pause state; landing flash shows the last locked cells.
