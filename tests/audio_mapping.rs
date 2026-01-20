use gpui_tetris::audio::sound_event_to_asset;
use gpui_tetris::game::state::SoundEvent;

#[test]
fn maps_sound_events_to_asset_keys() {
    assert_eq!(sound_event_to_asset(&SoundEvent::Move), Some("move"));
    assert_eq!(sound_event_to_asset(&SoundEvent::Rotate), Some("rotate"));
    assert_eq!(sound_event_to_asset(&SoundEvent::SoftDrop), Some("soft_drop"));
    assert_eq!(sound_event_to_asset(&SoundEvent::HardDrop), Some("hard_drop"));
    assert_eq!(sound_event_to_asset(&SoundEvent::Hold), Some("hold"));
    assert_eq!(sound_event_to_asset(&SoundEvent::LineClear(1)), Some("line_clear_1"));
    assert_eq!(sound_event_to_asset(&SoundEvent::LineClear(2)), Some("line_clear_2"));
    assert_eq!(sound_event_to_asset(&SoundEvent::LineClear(3)), Some("line_clear_3"));
    assert_eq!(sound_event_to_asset(&SoundEvent::LineClear(4)), Some("line_clear_4"));
    assert_eq!(sound_event_to_asset(&SoundEvent::GameOver), Some("game_over"));
}
