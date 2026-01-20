use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{spawn_position, Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};

#[test]
fn hold_sets_piece_and_spawns_next() {
    let mut state = GameState::new(1, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::I, 3, 0);
    state.active.rotation = Rotation::North;
    state.next_queue = vec![TetrominoType::O];

    state.apply_action(GameAction::Hold);

    assert_eq!(state.hold, Some(TetrominoType::I));
    assert_eq!(state.active.kind, TetrominoType::O);
    assert!(!state.can_hold);
}

#[test]
fn hold_swaps_with_held_piece() {
    let mut state = GameState::new(2, GameConfig::default());
    state.hold = Some(TetrominoType::T);
    state.active = Tetromino::new(TetrominoType::O, 4, 0);
    state.active.rotation = Rotation::North;
    state.can_hold = true;

    state.apply_action(GameAction::Hold);

    assert_eq!(state.hold, Some(TetrominoType::O));
    assert_eq!(state.active.kind, TetrominoType::T);
    let (spawn_x, spawn_y) = spawn_position();
    assert_eq!(state.active.x, spawn_x);
    assert_eq!(state.active.y, spawn_y);
    assert!(!state.can_hold);
}

#[test]
fn hold_is_once_per_spawn() {
    let mut state = GameState::new(3, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::I, 3, 0);
    state.next_queue = vec![TetrominoType::O];

    state.apply_action(GameAction::Hold);
    let hold_after_first = state.hold;
    let active_after_first = state.active.kind;

    state.apply_action(GameAction::Hold);

    assert_eq!(state.hold, hold_after_first);
    assert_eq!(state.active.kind, active_after_first);
}
