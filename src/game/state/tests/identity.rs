use super::super::*;
use crate::adapter::state_hash;
#[test]
fn state_hash_changes_when_board_changes() {
    let mut state = GameState::new(1, GameConfig::default());
    let before = state_hash(&state);
    state.board.cells[0][0] = crate::game::board::Cell {
        kind: Some(TetrominoType::I),
    };
    let after = state_hash(&state);
    assert_ne!(before, after);
}

#[test]
fn state_hash_covers_logical_and_scoring_identity() {
    let mut state = GameState::new(1, GameConfig::default());
    let before = state_hash(&state);
    state.logical_step += 1;
    assert_ne!(before, state_hash(&state));

    let before = state_hash(&state);
    state.combo += 1;
    assert_ne!(before, state_hash(&state));
}
