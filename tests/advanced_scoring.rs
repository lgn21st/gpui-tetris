use gpui_tetris::game::board::BOARD_HEIGHT;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState, Ruleset, TSpinKind};

#[test]
fn combo_bonus_increases_on_consecutive_clears() {
    let mut state = GameState::new(
        1,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.apply_line_clear(1, TSpinKind::None);
    state.apply_line_clear(1, TSpinKind::None);
    assert_eq!(state.score, 130);
    assert_eq!(state.combo, 1);
}

#[test]
fn back_to_back_applies_to_tetris() {
    let mut state = GameState::new(
        2,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.apply_line_clear(4, TSpinKind::None);
    state.apply_line_clear(4, TSpinKind::None);
    assert!(state.back_to_back);
    assert_eq!(state.score, 3050);
}

#[test]
fn t_spin_full_no_line_scores_and_sets_back_to_back_false() {
    let mut state = GameState::new(
        3,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.active = Tetromino::new(TetrominoType::T, 3, BOARD_HEIGHT as i32 - 3);
    state.active.rotation = Rotation::East;

    // Set last action rotate to enable T-spin detection.
    state.apply_action(GameAction::RotateCw);
    state.active = Tetromino::new(TetrominoType::T, 3, BOARD_HEIGHT as i32 - 3);
    state.active.rotation = Rotation::East;

    // Occupy three corners around the T center (x+1, y+1),
    // including both front corners for a full T-spin.
    state.board.cells[BOARD_HEIGHT - 3][5].filled = true;
    state.board.cells[BOARD_HEIGHT - 3][5].kind = Some(TetrominoType::L);
    state.board.cells[BOARD_HEIGHT - 1][5].filled = true;
    state.board.cells[BOARD_HEIGHT - 1][5].kind = Some(TetrominoType::L);
    state.board.cells[BOARD_HEIGHT - 3][3].filled = true;
    state.board.cells[BOARD_HEIGHT - 3][3].kind = Some(TetrominoType::L);

    state.apply_action(GameAction::HardDrop);

    assert_eq!(state.score, 400);
    assert!(!state.back_to_back);
}

#[test]
fn t_spin_mini_no_line_scores_less() {
    let mut state = GameState::new(
        4,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.active = Tetromino::new(TetrominoType::T, 3, BOARD_HEIGHT as i32 - 3);
    state.active.rotation = Rotation::East;

    state.apply_action(GameAction::RotateCw);
    state.active = Tetromino::new(TetrominoType::T, 3, BOARD_HEIGHT as i32 - 3);
    state.active.rotation = Rotation::East;

    state.board.cells[BOARD_HEIGHT - 3][3].filled = true;
    state.board.cells[BOARD_HEIGHT - 3][3].kind = Some(TetrominoType::L);
    state.board.cells[BOARD_HEIGHT - 1][3].filled = true;
    state.board.cells[BOARD_HEIGHT - 1][3].kind = Some(TetrominoType::L);
    state.board.cells[BOARD_HEIGHT - 1][5].filled = true;
    state.board.cells[BOARD_HEIGHT - 1][5].kind = Some(TetrominoType::L);

    state.apply_action(GameAction::HardDrop);

    assert_eq!(state.score, 100);
    assert!(!state.back_to_back);
}

#[test]
fn t_spin_mini_line_clear_scores() {
    let mut state = GameState::new(
        5,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.apply_line_clear(1, TSpinKind::Mini);
    assert_eq!(state.score, 200);

    let mut state = GameState::new(
        6,
        GameConfig {
            ruleset: Ruleset::Modern,
            ..GameConfig::default()
        },
    );
    state.apply_line_clear(2, TSpinKind::Mini);
    assert_eq!(state.score, 400);
}
