# Adapter Implementation Profile

Project-owned choices for the protocol whose normative package and version are
in the
[`lgn21st/tui-tetris` Adapter package](https://github.com/lgn21st/tui-tetris/tree/main/protocol/adapter).

## Runtime and bounds

- Pumped by a GPUI executor timer independently of drawing; listener and clients
  are nonblocking. Idle timeouts use a monotonic clock.
- Pump order: poll/apply commands, advance fixed steps, emit latest full
  observation. Rendering consumes the state independently.
- Place planning reuses core collision and SRS rules.
- At most 16 clients and 64 pending commands (`TETRIS_AI_MAX_PENDING`).
- Frames: 65,536-byte payload maximum; per-client response buffer: 262,144
  bytes; backpressure retry hint: 16 ms.
- Observations use one in-progress and one replaceable latest frame per client.
- A started outbound frame must finish before another frame begins. Unsent
  observations are replaceable; reliable responses have priority between frames.
- Per pump: at most 16 accepts, 16 KiB read and 64 decoded frames per client,
  64 game commands globally, and 64 KiB / 64 write calls per client.
- Events describe only the represented logical transition, capped at four.
  Repeated snapshots of the same logical_step retain the same events.
- The core next queue has at least five post-spawn entries; UI shows three and
  protocol publishes five.

## Control and cadence

- First eligible `auto`/`controller` client controls the game; explicit
  observers are never auto-promoted.
- Controller disconnect promotes the eligible lowest client id. Normal release
  leaves control unowned until an explicit claim.
- The 2,000 ms default inbound idle timeout applies to incomplete handshakes and
  controllers, not passive observers (`TETRIS_AI_IDLE_TIMEOUT_MS`).
- `TETRIS_AI_OBSERVATION_MS=0` emits each pump; positive values throttle. A new
  streaming client always receives an immediate full snapshot.

## Startup, logging, and security

- Enabled by default on `127.0.0.1:7777`; configure with `TETRIS_AI_HOST` and
  `TETRIS_AI_PORT`, or disable with `TETRIS_AI_DISABLED=1`/`true`.
- Adapter mode starts gameplay immediately. Focus loss never changes core pause
  state; it only clears held local input.
- Diagnostic logging is off by default; `TETRIS_AI_LOG_PATH=auto` selects
  `/tmp/tetris-ai-adapter-<unix-ms>.jsonl`, an explicit path selects that file,
  and empty disables logging. A 32-record queue feeds a worker; overflow drops
  diagnostics only. Files rotate at 8 MiB with one `.previous` backup. File
  errors disable that worker without blocking the Runtime.
- Log directions distinguish `recv`, `enqueue_response` and
  `enqueue_observation`; enqueue records do not prove delivery.
- The nonstandard redundant `board.kinds` extension has been removed; the
  normative `board.cells` array is the authoritative serialized board.
- `state_hash` is an implementation-specific observation identity, not a
  serialized checkpoint or a guarantee that hidden RNG/config state is equal.
- TCP has no authentication or TLS. Use non-loopback binding only on a trusted,
  access-controlled network.
