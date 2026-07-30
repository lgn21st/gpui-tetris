---
name: gpui-tetris-dev
description: "Build, audit, refactor, test, and document the Rust/GPUI Tetris application in this repository. Use for gameplay rules, timing, scoring, input, UI, audio, Adapter Protocol, dependency/toolchain upgrades, performance or resource bounds, macOS GPUI integration, and repository-wide reviews."
---

# GPUI Tetris Development

## Establish context

1. Read the repository `AGENTS.md` before acting.
2. Inspect `git status` and preserve unrelated or pre-existing changes.
3. Read only the project documents relevant to the task:
   - Architecture and ownership: `docs/ARCHITECTURE.md`
   - Gameplay semantics: `docs/rules-spec.md`
   - Dependencies and toolchain: `docs/dependencies.md`, `Cargo.toml`, and
     `rust-toolchain.toml`
   - Adapter conformance and local choices: `docs/adapter_acceptance.md` and
     `docs/adapter-implementation-profile.md`

Treat those repository files as the current-state authority. Do not duplicate
feature inventories in this skill.

## Keep ownership boundaries

- Keep deterministic gameplay rules and state transitions in `src/game/`.
- Keep GPUI lifecycle, rendering, and device handling in `src/ui/`.
- Route inputs through `GameAction`; do not let UI code mutate core state
  directly.
- Keep audio optional and non-fatal. Bound event queues and avoid blocking the
  game/render loop.
- Reuse core collision, rotation, and scoring rules from Adapter planning; do
  not maintain a second gameplay implementation.
- Make resource limits explicit for queues, buffers, catch-up loops, and event
  collections.

## Implement with evidence

1. Reproduce or specify the behavior with a focused test before changing the
   implementation when practical.
2. Make the smallest coherent change in the owning module.
3. Add boundary and failure-path coverage, especially for timers, arithmetic,
   protocol input, and bounded resources.
4. Update `AGENTS.md` and the relevant project documents when commands,
   structure, behavior, dependencies, or operational choices change.
5. Run focused tests first, then the full gates below.

## Follow the Adapter Protocol authority

Before Adapter work, read the authoritative package in this exact order:

1. `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter/VERSION`
2. `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter/CHANGELOG.md`
3. `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter/SPEC.md`
4. `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter/schema.json`
5. `/Users/daniel/workspace/learn/tui-tetris/protocol/adapter/profiles/tcp-json-lines.md`

- Use TDD for protocol migrations.
- Maintain the requirement-to-implementation-to-test matrix in
  `docs/adapter_acceptance.md`.
- Keep queue capacity, threading, scheduling, logging, buffering, security, and
  startup decisions in `docs/adapter-implementation-profile.md`.
- Do not create a local `adapter.md` or prose copy of the normative protocol.

## Handle dependencies and macOS deliberately

- Use the stable toolchain selected by `rust-toolchain.toml` and respect the
  package MSRV.
- Verify current upstream versions before dependency changes.
- Do not assume a vendored workaround is still required. Inspect
  `[patch.crates-io]`, `Cargo.lock`, and `docs/dependencies.md`; validate removal
  in a temporary copy before deleting local patches.
- When changing GPUI or macOS graphics/font dependencies, inspect the resolved
  `core-text`/`core-graphics` graph and compile all targets.
- Use GPUI runtime shaders for CI-like checks on machines without the Metal
  compiler.
- Record unavoidable upstream advisories or future-incompatibility warnings
  with reachability and ownership, rather than silently ignoring them.

## Verification gates

Run focused tests, then every repository gate listed in `AGENTS.md`.

Adapter TCP tests bind loopback sockets and may require sandbox approval. After
dependency changes, also run `cargo audit --no-fetch --no-yanked` and explain
any non-zero result. For UI changes, launch the app and visually verify the
affected flow when the environment supports it.
