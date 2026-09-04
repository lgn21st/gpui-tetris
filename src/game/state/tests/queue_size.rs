use crate::game::state::{GameConfig, GameState};

#[test]
fn next_queue_keeps_minimum_size() {
    let state = GameState::new(1, GameConfig::default());
    assert!(state.next_queue.len() >= 5);
}

#[test]
fn spawn_next_refills_to_keep_preview_buffer() {
    let mut state = GameState::new(2, GameConfig::default());
    state.next_queue = vec![state.active.kind];
    state.spawn_next();
    assert!(state.next_queue.len() >= 5);
}
