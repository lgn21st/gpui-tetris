# gpui-tetris

Desktop Tetris for macOS, written in Rust with GPUI. It provides classic and
modern scoring, SRS rotation, keyboard/controller input, resizable rendering,
optional WAV effects, and a TCP adapter for AI clients.

## Controls

| Input | Action |
| --- | --- |
| Left/Right, Down, Up | Move, soft drop, rotate clockwise |
| Space, C, P, R | Hard drop, hold, pause, restart |
| Enter, S, M | Start, settings, mute |
| `+`/`-`, `0` | Adjust or reset SFX volume |
| Cmd+Ctrl+F | Toggle fullscreen |

Xbox defaults: D-pad/stick moves, A/B rotates, X holds, Y hard-drops,
Start pauses, and Select/Back restarts.

## Development

The repository selects Rust stable with rustfmt and Clippy; the package MSRV is
1.97.1.

```bash
cargo run
```

Core rules live in `src/game/`; GPUI lifecycle, input, and rendering live in
`src/ui/`; the AI transport lives in `src/adapter/`. See
`docs/ARCHITECTURE.md`, `docs/rules-spec.md`, and `docs/dependencies.md` for the
maintained contracts. Contributor workflow and verification commands are in
`AGENTS.md`.

## AI Adapter

The server implements Tetris AI Adapter Protocol 3.0.0 over JSON Lines TCP.
The normative protocol is
`/Users/daniel/workspace/learn/tui-tetris/protocol/adapter`; conformance evidence
is in `docs/adapter_acceptance.md`, while queueing, scheduling, logging, startup,
and security choices are in `docs/adapter-implementation-profile.md`.

It binds `127.0.0.1:7777` by default. Configure it with the `TETRIS_AI_*`
variables documented in the implementation profile. The transport has no TLS
or authentication; keep it on loopback unless the network is trusted.

## Assets and packaging

`assets/icon.svg` is the editable icon source and `assets/icon.icns` is the
macOS bundle artifact. Run `cargo bundle --release` with `cargo-bundle` to build
an app bundle. Optional effects are loaded from `assets/sfx/`; attribution and
source mapping are in `docs/audio_assets.md`.

## License

Licensed under the [MIT License](LICENSE).
