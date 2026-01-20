use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn next_queue_keeps_minimum_size() {
    let state = GameState::new(1, GameConfig::default());
    assert!(state.next_queue.len() >= 5);
}

#[test]
fn spawn_next_keeps_queue_filled() {
    let mut state = GameState::new(2, GameConfig::default());
    state.spawn_next();
    assert!(state.next_queue.len() >= 4);
}
