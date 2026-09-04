use gpui_tetris::game::{
    input::GameAction,
    state::{GameConfig, GameState},
};

#[test]
fn empty_restart_preserves_board_identity_but_starts_an_episode() {
    let mut state = GameState::new(1, GameConfig::default());
    let revision = state.board_revision();
    let episode = state.episode_id();
    state.reset_with_seed(1);
    assert_eq!(state.board_revision(), revision);
    assert_eq!(state.episode_id(), episode + 1);
    state.apply_action(GameAction::HardDrop);
    let revision = state.board_revision();
    state.reset_with_seed(1);
    assert_ne!(state.board_revision(), revision);
}

#[test]
fn events_do_not_leak_into_the_next_logical_transition() {
    let mut state = GameState::new(1, GameConfig::default());
    state.apply_action(GameAction::HardDrop);
    state.tick(16, false);
    assert!(state.transition().events.is_empty());
}
