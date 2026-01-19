use gpui_tetris::game::board::{Board, BOARD_WIDTH};
use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};
use gpui_tetris::game::input::GameAction;

#[test]
fn rotate_kicks_inside_left_wall() {
    let mut state = GameState::new(1, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::L, 0, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::RotateCw);
    assert!(state.active.x >= 0);
}

#[test]
fn rotate_kicks_inside_right_wall() {
    let mut state = GameState::new(2, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::J, BOARD_WIDTH as i32 - 1, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::RotateCw);
    assert!(state.active.x <= BOARD_WIDTH as i32 - 1);
}
