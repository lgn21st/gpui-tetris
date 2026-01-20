use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};
use gpui_tetris::game::board::BOARD_HEIGHT;

#[test]
fn tick_advances_piece_after_drop_interval() {
    let config = GameConfig {
        base_drop_ms: 500,
        ..GameConfig::default()
    };
    let mut state = GameState::new(1, config);
    let start_y = state.active.y;

    state.tick(499, false);
    assert_eq!(state.active.y, start_y);

    state.tick(1, false);
    assert_eq!(state.active.y, start_y + 1);
}

#[test]
fn tick_uses_soft_drop_interval() {
    let config = GameConfig {
        base_drop_ms: 1000,
        soft_drop_multiplier: 10,
        soft_drop_grace_ms: 200,
        ..GameConfig::default()
    };
    let mut state = GameState::new(2, config);
    let start_y = state.active.y;

    state.activate_soft_drop();
    state.tick(99, false);
    assert_eq!(state.active.y, start_y);

    state.tick(1, false);
    assert_eq!(state.active.y, start_y + 1);
}

#[test]
fn tick_locks_piece_after_lock_delay() {
    let config = GameConfig {
        base_drop_ms: 1000,
        lock_delay_ms: 500,
        ..GameConfig::default()
    };
    let mut state = GameState::new(3, config);
    state.active = Tetromino::new(TetrominoType::O, 3, BOARD_HEIGHT as i32 - 2);
    state.active.rotation = Rotation::North;

    state.tick(400, false);
    assert!(!state.game_over);
    assert_eq!(state.lock_timer_ms, 400);

    state.tick(200, false);
    assert_eq!(state.lock_timer_ms, 0);
    assert!(state.board.cells[BOARD_HEIGHT - 1][4].filled);
}

#[test]
fn tick_does_not_advance_when_paused() {
    let config = GameConfig {
        base_drop_ms: 500,
        ..GameConfig::default()
    };
    let mut state = GameState::new(4, config);
    state.paused = true;
    let start_y = state.active.y;

    state.tick(1000, false);
    assert_eq!(state.active.y, start_y);
}

#[test]
fn soft_drop_expires_after_grace_period() {
    let config = GameConfig {
        soft_drop_grace_ms: 100,
        ..GameConfig::default()
    };
    let mut state = GameState::new(5, config);
    state.activate_soft_drop();
    assert!(state.is_soft_drop_active());

    state.tick(100, false);
    assert!(!state.is_soft_drop_active());
}

#[test]
fn line_clear_timer_pauses_gravity() {
    let mut state = GameState::new(6, GameConfig::default());
    state.line_clear_timer_ms = 200;
    let start_y = state.active.y;

    state.tick(100, false);
    assert_eq!(state.active.y, start_y);

    state.tick(200, false);
    assert!(state.active.y >= start_y);
}
