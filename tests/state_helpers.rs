use gpui_tetris::game::board::BOARD_HEIGHT;
use gpui_tetris::game::pieces::{Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn line_clear_active_reflects_timer() {
    let mut state = GameState::new(1, GameConfig::default());
    assert!(!state.is_line_clear_active());
    state.line_clear_timer_ms = 10;
    assert!(state.is_line_clear_active());
}

#[test]
fn lock_reset_remaining_uses_limit_and_count() {
    let mut state = GameState::new(2, GameConfig::default());
    state.lock_reset_limit = 3;
    state.lock_reset_count = 1;
    assert_eq!(state.lock_reset_remaining(), 2);
}

#[test]
fn grounded_state_tracks_piece_support() {
    let mut state = GameState::new(3, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::O, 4, 0);
    assert!(!state.is_grounded());

    state.active = Tetromino::new(TetrominoType::O, 4, BOARD_HEIGHT as i32 - 2);
    assert!(state.is_grounded());
}
