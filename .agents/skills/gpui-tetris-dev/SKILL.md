---
name: gpui-tetris-dev
description: "Build, audit, refactor, test, and document this Rust/GPUI Tetris application, including its Adapter Protocol integration."
---

# GPUI Tetris Development

## Start from repository authority

1. Read `AGENTS.md` and inspect `git status`.
2. Read only the documents relevant to the change:
   - architecture: `docs/ARCHITECTURE.md`
   - gameplay: `docs/rules-spec.md`
   - dependencies: `docs/dependencies.md`, `Cargo.toml`, `Cargo.lock`, and
     `rust-toolchain.toml`
   - Adapter: `docs/adapter_acceptance.md` and
     `docs/adapter-implementation-profile.md`

Do not repeat dependency, toolchain, or protocol versions in prose; their
machine-readable sources own them. Avoid audit snapshots and feature lists that
will drift.

## Preserve boundaries

- Keep deterministic rules in `src/game/`, GPUI concerns in `src/ui/`, and
  optional bounded audio in `src/audio.rs`.
- Route all mutation through `GameAction`.
- Reuse core rules for Adapter planning.
- Bound queues, buffers, event collections, repeats, and catch-up loops.
- Add focused boundary and failure-path tests before behavior changes when
  practical, then update only the owning documentation.

## Adapter authority

Use the
[`lgn21st/tui-tetris` Adapter package](https://github.com/lgn21st/tui-tetris/tree/main/protocol/adapter)
as the authority. Before Adapter work, read in order:

1. [`VERSION`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/VERSION)
2. [`CHANGELOG.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/CHANGELOG.md)
3. [`SPEC.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/SPEC.md)
4. [`schema.json`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/schema.json)
5. [`profiles/tcp-json-lines.md`](https://github.com/lgn21st/tui-tetris/blob/main/protocol/adapter/profiles/tcp-json-lines.md)

Use TDD for migrations. Keep the compact requirement/implementation/difference/
evidence matrix in `docs/adapter_acceptance.md`. Keep queueing, threading,
scheduling, logging, buffering, security, and startup choices in
`docs/adapter-implementation-profile.md`. Never copy the normative protocol or
create a local `adapter.md`.

## Dependencies and verification

- Prefer upstream crates; add a local patch only for a reproduced issue with a
  documented removal condition.
- For GPUI/macOS dependency changes, inspect the resolved graphics/font graph
  and compile all targets with runtime shaders.
- Record upstream advisories by reachability and owner, without copying version
  snapshots.
- Run every gate in `AGENTS.md`. After dependency changes also run
  `cargo audit --no-fetch --no-yanked`; Adapter TCP tests may need loopback
  permission. Visually verify UI changes when possible.
