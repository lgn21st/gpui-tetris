use gpui_tetris::game::board::{Board, BOARD_HEIGHT};
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn move_left_stops_at_wall() {
    let config = GameConfig::default();
    let mut state = GameState::new(1, config);
    state.active = Tetromino::new(TetrominoType::I, 0, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::MoveLeft);
    assert_eq!(state.active.x, 0);
}

#[test]
fn rotate_changes_orientation_when_clear() {
    let config = GameConfig::default();
    let mut state = GameState::new(2, config);
    state.active = Tetromino::new(TetrominoType::T, 3, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::RotateCw);
    assert_eq!(state.active.rotation, Rotation::East);
}

#[test]
fn hard_drop_locks_piece() {
    let config = GameConfig::default();
    let mut state = GameState::new(3, config);
    state.active = Tetromino::new(TetrominoType::O, 4, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::HardDrop);
    let filled = state
        .board
        .cells
        .iter()
        .flatten()
        .filter(|cell| cell.filled)
        .count();
    assert_eq!(filled, 4);
}

#[test]
fn move_right_stops_at_occupied_cell() {
    let config = GameConfig::default();
    let mut state = GameState::new(4, config);
    state.active = Tetromino::new(TetrominoType::O, 3, 0);
    state.active.rotation = Rotation::North;
    state.board.cells[0][5].filled = true;
    state.board.cells[0][5].kind = Some(TetrominoType::I);

    state.apply_action(GameAction::MoveRight);
    assert_eq!(state.active.x, 3);
}

#[test]
fn hard_drop_stops_above_blocker() {
    let config = GameConfig::default();
    let mut state = GameState::new(5, config);
    state.active = Tetromino::new(TetrominoType::O, 3, 0);
    state.active.rotation = Rotation::North;
    state.board.cells[19][4].filled = true;
    state.board.cells[19][4].kind = Some(TetrominoType::Z);

    state.apply_action(GameAction::HardDrop);

    assert!(state.board.cells[18][4].filled);
    assert!(state.board.cells[19][4].filled);
}

#[test]
fn ghost_blocks_reach_bottom() {
    let mut state = GameState::new(4, GameConfig::default());
    state.board = Board::new();
    state.active = Tetromino::new(TetrominoType::I, 3, 0);
    state.active.rotation = Rotation::South;

    let ghost_blocks = state.ghost_blocks();
    let max_y = ghost_blocks.iter().map(|(_, y)| *y).max().unwrap();
    assert_eq!(max_y, BOARD_HEIGHT as i32 - 1);
}
