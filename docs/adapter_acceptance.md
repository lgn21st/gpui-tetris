# Adapter Protocol Conformance

Authority: the
[`lgn21st/tui-tetris` Adapter package](https://github.com/lgn21st/tui-tetris/tree/main/protocol/adapter).
Its `VERSION`, changelog, specification, schema, and TCP profile are the only
normative sources; this file records implementation coverage without copying
them.

| Protocol requirement | Current implementation | Difference | Test evidence |
| --- | --- | --- | --- |
| Negotiate the current major version, validate hello identity/format, and enforce client sequence | Typed handshake and per-connection sequence validation in `adapter/transport.rs` | Aligned | Handshake and sequence cases in `tests/adapter_tcp.rs` |
| Assign one controller and implement claim, release, promotion, and observer rules | Connection roles and deterministic promotion are transport-owned | Aligned | Role lifecycle cases in `tests/adapter_tcp.rs` |
| Validate action counts, restart coupling, place coordinates, and rotation values | Typed parsing rejects invalid commands before mutation | Aligned | Validation cases plus adapter decoding unit tests |
| Apply commands authoritatively; keep seeded restart deterministic and failed place atomic | Actions reuse core state transitions; place plans against a clone before commit | Aligned | Command, restart, and place cases in `tests/adapter_tcp.rs` |
| Publish complete observations with stable identities, queue, events, and state hash | `adapter/mod.rs` maps core state and drains bounded causal events | Aligned | Observation, identity, queue, event, and hash tests |
| Bound frames and outbound data without allowing slow clients to stall the game | Nonblocking capped buffers with latest-observation coalescing | Aligned | Payload-boundary, backpressure, and isolation cases |
| Preserve snapshot cadence, passive observers, reconnects, and controller liveness | Immediate initial snapshot, configurable throttling, role-aware idle handling | Aligned | Snapshot, idle, and reconnect cases |
| Pass portable black-box behavior outside the Rust test harness | Release application accepts the authority verifier | Aligned | `adapter_verify.py all` |

Run the repository gates in `AGENTS.md`; the TCP suite may require loopback
socket permission.
