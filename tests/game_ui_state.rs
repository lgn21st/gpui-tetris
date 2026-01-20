use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn pause_toggles_state() {
    let mut state = GameState::new(1, GameConfig::default());
    assert!(!state.paused);
    state.apply_action(GameAction::Pause);
    assert!(state.paused);
    state.apply_action(GameAction::Pause);
    assert!(!state.paused);
}

#[test]
fn restart_resets_score_and_flags() {
    let mut state = GameState::new(2, GameConfig::default());
    state.score = 400;
    state.lines = 12;
    state.level = 2;
    state.paused = true;
    state.game_over = true;

    state.apply_action(GameAction::Restart);

    assert_eq!(state.score, 0);
    assert_eq!(state.lines, 0);
    assert_eq!(state.level, 0);
    assert!(!state.paused);
    assert!(!state.game_over);
}

#[test]
fn game_over_when_spawn_blocked() {
    let mut state = GameState::new(3, GameConfig::default());
    state.board.cells[0][4].filled = true;
    state.board.cells[0][4].kind = Some(gpui_tetris::game::pieces::TetrominoType::I);
    state.next_queue = vec![gpui_tetris::game::pieces::TetrominoType::O];

    state.spawn_next();

    assert!(state.game_over);
}

#[test]
fn game_over_blocks_actions_except_restart() {
    let mut state = GameState::new(4, GameConfig::default());
    state.game_over = true;
    let start_x = state.active.x;

    state.apply_action(GameAction::MoveLeft);
    assert_eq!(state.active.x, start_x);

    state.apply_action(GameAction::Restart);
    assert!(!state.game_over);
}
