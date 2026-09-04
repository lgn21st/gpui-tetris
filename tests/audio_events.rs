#![cfg(feature = "audio")]

use gpui_tetris::audio::sound_event_spec;
use gpui_tetris::game::state::SoundEvent;

#[test]
fn maps_sound_events_to_assets_and_gains() {
    for (event, expected) in [
        (SoundEvent::Move, ("move", 0.25)),
        (SoundEvent::Rotate, ("rotate", 0.35)),
        (SoundEvent::SoftDrop, ("soft_drop", 0.2)),
        (SoundEvent::HardDrop, ("hard_drop", 0.6)),
        (SoundEvent::Hold, ("hold", 0.5)),
        (SoundEvent::LineClear(1), ("line_clear_1", 0.6)),
        (SoundEvent::LineClear(2), ("line_clear_2", 0.7)),
        (SoundEvent::LineClear(3), ("line_clear_3", 0.8)),
        (SoundEvent::LineClear(4), ("line_clear_4", 0.9)),
        (SoundEvent::GameOver, ("game_over", 0.8)),
    ] {
        assert_eq!(sound_event_spec(&event), expected);
    }
}
