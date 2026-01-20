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

#[test]
fn rotate_kicks_around_blocker() {
    let mut state = GameState::new(3, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::T, 0, 0);
    state.active.rotation = Rotation::North;

    state.board.cells[0][1].filled = true;
    state.board.cells[0][1].kind = Some(TetrominoType::I);

    state.apply_action(GameAction::RotateCw);
    assert_eq!(state.active.rotation, Rotation::East);
    assert_ne!(state.active.x, 0);
}

#[test]
fn rotate_fails_when_all_kicks_blocked() {
    let mut state = GameState::new(4, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::T, 0, 0);
    state.active.rotation = Rotation::North;

    for (x, y) in [(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1)].iter() {
        state.board.cells[*y][*x].filled = true;
        state.board.cells[*y][*x].kind = Some(TetrominoType::O);
    }

    state.apply_action(GameAction::RotateCw);
    assert_eq!(state.active.rotation, Rotation::North);
}

#[test]
fn rotate_kicks_up_from_floor() {
    let mut state = GameState::new(5, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::I, 3, 17);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::RotateCw);
    assert_eq!(state.active.rotation, Rotation::East);
    assert!(state.active.y < 17);
}
