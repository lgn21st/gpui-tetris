use gpui_tetris::audio::sound_event_gain;
use gpui_tetris::game::state::SoundEvent;

#[test]
fn assigns_gain_per_event() {
    assert!(sound_event_gain(&SoundEvent::Move) < sound_event_gain(&SoundEvent::Rotate));
    assert!(sound_event_gain(&SoundEvent::HardDrop) > sound_event_gain(&SoundEvent::SoftDrop));
    assert!(
        sound_event_gain(&SoundEvent::LineClear(4)) >= sound_event_gain(&SoundEvent::LineClear(1))
    );
}
