use crate::game::board::BOARD_HEIGHT;
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType};
use crate::game::state::{GameConfig, GameState, RulesConfig, Ruleset, TSpinKind};

#[test]
fn soft_drop_awards_point_per_cell() {
    let mut state = GameState::new(1, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::O, 3, 0);
    state.active.rotation = Rotation::North;
    state.score = 0;

    state.apply_action(GameAction::SoftDrop);

    assert_eq!(state.active.y, 1);
    assert_eq!(state.score, 1);
}

#[test]
fn line_clear_sets_timer() {
    let mut state = GameState::new(6, GameConfig::default());
    state.apply_line_clear(1, TSpinKind::None);
    assert!(state.line_clear_timer_ms > 0);
}

#[test]
fn hard_drop_awards_two_points_per_cell() {
    let mut state = GameState::new(2, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::O, 3, 0);
    state.active.rotation = Rotation::North;
    state.score = 0;

    state.apply_action(GameAction::HardDrop);

    let expected_rows = (BOARD_HEIGHT as i32 - 2) as u32;
    assert_eq!(state.score, expected_rows * 2);
}

#[test]
fn drop_interval_decreases_with_level_and_has_floor() {
    let mut state = GameState::new(3, GameConfig::default());
    state.level = 0;
    let base_interval = state.drop_interval_ms(false);

    state.level = 5;
    let faster = state.drop_interval_ms(false);
    assert!(faster < base_interval);

    state.level = 100;
    assert_eq!(state.drop_interval_ms(false), 120);
}

#[test]
fn scoring_saturates_instead_of_overflowing() {
    let rules = RulesConfig {
        classic_line_scores: [u32::MAX; 4],
        ..RulesConfig::default()
    };
    let mut state = GameState::new(
        8,
        GameConfig {
            rules,
            ..GameConfig::default()
        },
    );
    state.score = u32::MAX - 1;
    state.level = u32::MAX;

    state.apply_line_clear(1, TSpinKind::None);

    assert_eq!(state.score, u32::MAX);
    assert_eq!(state.lines, 1);
}

#[test]
fn zero_b2b_denominator_is_handled_without_panicking() {
    let rules = RulesConfig {
        b2b_bonus_den: 0,
        ..RulesConfig::default()
    };
    let mut state = GameState::new(
        9,
        GameConfig {
            ruleset: Ruleset::Modern,
            rules,
            ..GameConfig::default()
        },
    );
    state.back_to_back = true;

    state.apply_line_clear(4, TSpinKind::None);

    assert!(state.score > 0);
}
