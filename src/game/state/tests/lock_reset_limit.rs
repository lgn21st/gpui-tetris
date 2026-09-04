use crate::game::board::BOARD_HEIGHT;
use crate::game::input::GameAction;
use crate::game::pieces::{Tetromino, TetrominoType};
use crate::game::state::{GameConfig, GameState};

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
    assert!(
        state
            .board
            .cells
            .iter()
            .flatten()
            .any(|cell| cell.kind.is_some())
    );
}

#[test]
fn becoming_airborne_restores_lock_reset_budget() {
    let mut state = GameState::new(2, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::O, 3, BOARD_HEIGHT as i32 - 3);
    state.board.cells[BOARD_HEIGHT - 1][4].kind = Some(crate::game::pieces::TetrominoType::I);
    state.lock_reset_count = state.lock_reset_limit;
    state.lock_timer_ms = 300;

    state.apply_action(GameAction::MoveRight);
    assert!(!state.is_grounded());
    assert_eq!(state.lock_reset_count, 0);

    state.tick(16, false);
    assert_eq!(state.lock_reset_count, 0);
}
