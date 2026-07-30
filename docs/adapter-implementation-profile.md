# Adapter Implementation Profile

Project-owned choices for Tetris AI Adapter Protocol 3.0.0. The normative
package is `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter`.

## Runtime and bounds

- Embedded in the GPUI thread; listener and clients are nonblocking.
- Frame order: poll/apply commands, advance fixed steps, emit latest full
  observation, render.
- Place planning reuses core collision and SRS rules.
- At most 16 clients and 64 pending commands (`TETRIS_AI_MAX_PENDING`).
- Frames: 65,536-byte payload maximum; per-client response buffer: 262,144
  bytes; backpressure retry hint: 16 ms.
- Observations use one in-progress and one replaceable latest frame per client.
- Gameplay events remain causal and are capped at four per observation.
- The core next queue has at least five post-spawn entries; UI shows three and
  protocol publishes five.

## Control and cadence

- First eligible `auto`/`controller` client controls the game; explicit
  observers are never auto-promoted.
- Controller disconnect promotes the eligible lowest client id. Normal release
  leaves control unowned until an explicit claim.
- The 2,000 ms default inbound idle timeout applies to incomplete handshakes and
  controllers, not passive observers (`TETRIS_AI_IDLE_TIMEOUT_MS`).
- `TETRIS_AI_OBSERVATION_MS=0` emits each frame; positive values throttle. A new
  streaming client always receives an immediate full snapshot.

## Startup, logging, and security

- Enabled by default on `127.0.0.1:7777`; configure with `TETRIS_AI_HOST` and
  `TETRIS_AI_PORT`, or disable with `TETRIS_AI_DISABLED=1`/`true`.
- Adapter mode starts gameplay immediately and disables focus-loss auto-pause.
- `TETRIS_AI_LOG_PATH=auto` appends JSON Lines to
  `/tmp/tetris-ai-adapter-<unix-ms>.jsonl`; empty disables logging. Logging is
  synchronous and non-fatal.
- TCP has no authentication or TLS. Use non-loopback binding only on a trusted,
  access-controlled network.
