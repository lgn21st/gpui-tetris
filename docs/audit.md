# Audit Summary

Last full audit: 2026-07-30. Scope: rules, timing, input, UI, audio, Adapter
Protocol 3.0.0, resource bounds, dependencies, documentation, and tests.

| Area | Resolved findings | Evidence |
| --- | --- | --- |
| Rules | Corrected positive-down SRS kicks, lock-reset lifetime, pause elapsed-time accounting, B2B semantics, and saturating scoring | Rotation, timing, lock-reset, and scoring suites |
| Responsiveness | Bounded frame catch-up, input repeats, sound/event queues, audio voices, Adapter buffers, and slow-client handling | Boundary tests and 32 Adapter TCP tests |
| Reliability | Removed production unwraps, rejected out-of-schema rotations, propagated WAV errors, and suppressed sounds for failed actions | Clippy and focused regression tests |
| Adapter | Migrated 2.0.0 to 3.0.0, reused core placement rules, expanded authoritative identities/hash, and documented implementation policy | `adapter_acceptance.md` |
| Toolchain | Set Rust 1.97.1 MSRV, upgraded direct/compatible dependencies, migrated CPAL 0.18, and removed 8.7 MB of verified-unneeded vendor code | Full test and unvendored Clippy gates |
| Release | Built a 7.3 MB arm64 `.app`; verified icon, ten bundled WAVs, detached `/tmp` launch, and authority-client readiness | `cargo bundle --release` and black-box client |
| Performance | Five-second release sample: 49.8 MB physical footprint; active stacks were GPUI drawing while audio/logging waited. Logging on/off determinism runs both took 0.43 s | macOS `sample` and timed black-box runs |

Intentional choices: the deterministic queue is not a seven-bag; excess
wall-clock catch-up and sound events are dropped; Adapter logging is
synchronous/non-fatal; unauthenticated TCP defaults to loopback. Current
dependency risks live in `dependencies.md`, Adapter evidence in
`adapter_acceptance.md`, and verification commands in `AGENTS.md`.
