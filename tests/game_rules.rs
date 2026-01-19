use gpui_tetris::game::board::{Board, BOARD_HEIGHT, BOARD_WIDTH};
use gpui_tetris::game::pieces::{spawn_position, Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn rotation_cycles() {
    assert_eq!(Rotation::North.cw(), Rotation::East);
    assert_eq!(Rotation::East.cw(), Rotation::South);
    assert_eq!(Rotation::South.cw(), Rotation::West);
    assert_eq!(Rotation::West.cw(), Rotation::North);

    assert_eq!(Rotation::North.ccw(), Rotation::West);
    assert_eq!(Rotation::West.ccw(), Rotation::South);
    assert_eq!(Rotation::South.ccw(), Rotation::East);
    assert_eq!(Rotation::East.ccw(), Rotation::North);
}

#[test]
fn spawn_position_is_classic() {
    assert_eq!(spawn_position(), (3, 0));
}

#[test]
fn can_place_and_collision_detection() {
    let mut board = Board::new();
    let piece = Tetromino::new(TetrominoType::O, 0, 0);

    assert!(board.can_place(&piece, piece.x, piece.y, piece.rotation));

    board.cells[0][1].filled = true;
    board.cells[0][1].kind = Some(TetrominoType::I);
    assert!(!board.can_place(&piece, piece.x, piece.y, piece.rotation));
}

#[test]
fn lock_piece_marks_cells() {
    let mut board = Board::new();
    let mut piece = Tetromino::new(TetrominoType::O, 0, 0);
    piece.rotation = Rotation::North;

    board.lock_piece(&piece);

    for (dx, dy) in piece.blocks(piece.rotation) {
        let x = (piece.x + dx) as usize;
        let y = (piece.y + dy) as usize;
        assert!(board.cells[y][x].filled);
        assert_eq!(board.cells[y][x].kind, Some(TetrominoType::O));
    }
}

#[test]
fn clear_lines_removes_full_row() {
    let mut board = Board::new();
    let y = BOARD_HEIGHT - 1;

    for x in 0..BOARD_WIDTH {
        board.cells[y][x].filled = true;
        board.cells[y][x].kind = Some(TetrominoType::T);
    }

    let cleared = board.clear_lines();
    assert_eq!(cleared, 1);
    assert!(board.cells[y].iter().all(|cell| !cell.filled));
}

#[test]
fn apply_line_clear_updates_score_and_level() {
    let config = GameConfig::default();
    let mut state = GameState::new(1, config);

    state.apply_line_clear(1);
    assert_eq!(state.lines, 1);
    assert_eq!(state.score, 40);
    assert_eq!(state.level, 0);

    state.lines = 9;
    state.apply_line_clear(1);
    assert_eq!(state.lines, 10);
    assert_eq!(state.level, 1);
    assert_eq!(state.score, 80);
}
