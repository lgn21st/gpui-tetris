# Audio Assets (Free Sources)

This project expects WAV files in `assets/sfx/` with the names listed in `README.md`.
To keep licensing clean, prefer public-domain or permissive licenses (CC0, CC-BY).

Current pack in use:
- Kenney “Interface Sounds” v1.0 (CC0). License text: `docs/kenney_interface_sounds_LICENSE.txt`.

Conversion notes:
- Source pack ships as OGG; converted to 44.1kHz stereo PCM WAV via `ffmpeg`.

Current mapping:
- `move.wav`: `Audio/click_001.ogg`
- `rotate.wav`: `Audio/click_002.ogg`
- `soft_drop.wav`: `Audio/tick_001.ogg`
- `hard_drop.wav`: `Audio/drop_001.ogg`
- `hold.wav`: `Audio/toggle_001.ogg`
- `line_clear_1.wav`: `Audio/confirmation_001.ogg`
- `line_clear_2.wav`: `Audio/confirmation_002.ogg`
- `line_clear_3.wav`: `Audio/confirmation_003.ogg`
- `line_clear_4.wav`: `Audio/confirmation_004.ogg`
- `game_over.wav`: `Audio/error_001.ogg`

When adding files:
- Keep them short and trimmed (under 0.5s for moves/rotations).
- Normalize levels so the mixer doesn’t clip.
- Use 44.1kHz stereo if possible; mono is fine.
