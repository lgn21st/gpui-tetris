use gpui_tetris::game::board::BOARD_HEIGHT;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn grounded_moves_reset_lock_delay_until_limit() {
    let config = GameConfig {
        lock_delay_ms: 1000,
        lock_reset_limit: 1,
        ..GameConfig::default()
    };
    let mut state = GameState::new(1, config);
    state.active = Tetromino::new(TetrominoType::O, 4, BOARD_HEIGHT as i32 - 2);
    state.lock_timer_ms = 900;

    state.apply_action(GameAction::MoveLeft);
    assert_eq!(state.lock_timer_ms, 0);

    state.lock_timer_ms = 900;
    state.apply_action(GameAction::MoveRight);
    assert_eq!(state.lock_timer_ms, 900);

    state.tick(200, false);
    assert!(state.board.cells.iter().flatten().any(|cell| cell.filled));
}
