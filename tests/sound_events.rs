use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState, SoundEvent};

#[test]
fn emits_sound_events_for_actions() {
    let mut state = GameState::new(1, GameConfig::default());
    state.active = Tetromino::new(TetrominoType::O, 3, 0);
    state.active.rotation = Rotation::North;

    state.apply_action(GameAction::MoveLeft);
    state.apply_action(GameAction::RotateCw);
    state.apply_action(GameAction::SoftDrop);
    state.apply_action(GameAction::HardDrop);

    let events = state.take_sound_events();
    assert!(events.contains(&SoundEvent::Move));
    assert!(events.contains(&SoundEvent::Rotate));
    assert!(events.contains(&SoundEvent::SoftDrop));
    assert!(events.contains(&SoundEvent::HardDrop));
}

#[test]
fn emits_line_clear_sound() {
    let mut state = GameState::new(2, GameConfig::default());
    state.apply_line_clear(2, false);
    let events = state.take_sound_events();
    assert!(events.contains(&SoundEvent::LineClear(2)));
}

#[test]
fn emits_game_over_sound_on_spawn_blocked() {
    let mut state = GameState::new(3, GameConfig::default());
    state.board.cells[0][4].filled = true;
    state.board.cells[0][4].kind = Some(TetrominoType::I);
    state.next_queue = vec![TetrominoType::O];

    state.spawn_next();

    let events = state.take_sound_events();
    assert!(events.contains(&SoundEvent::GameOver));
}
