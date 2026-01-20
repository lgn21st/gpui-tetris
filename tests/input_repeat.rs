use gpui_tetris::game::input::{RepeatConfig, RepeatState};

#[test]
fn repeat_press_fires_once_until_release() {
    let config = RepeatConfig { das_ms: 100, arr_ms: 50 };
    let mut state = RepeatState::new();

    assert!(state.press());
    assert!(!state.press());
    assert_eq!(state.tick(100, &config), 0);
    assert_eq!(state.tick(50, &config), 1);

    state.release();
    assert!(state.press());
}

#[test]
fn repeat_fires_multiple_steps_after_das() {
    let config = RepeatConfig { das_ms: 100, arr_ms: 50 };
    let mut state = RepeatState::new();

    state.press();
    assert_eq!(state.tick(200, &config), 2);
    assert_eq!(state.tick(100, &config), 2);
}
