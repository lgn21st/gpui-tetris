use gpui_tetris::game::input::{key_to_action, GameAction};

#[test]
fn key_to_action_maps_supported_keys() {
    assert_eq!(key_to_action("left"), Some(GameAction::MoveLeft));
    assert_eq!(key_to_action("right"), Some(GameAction::MoveRight));
    assert_eq!(key_to_action("down"), Some(GameAction::SoftDrop));
    assert_eq!(key_to_action("up"), Some(GameAction::RotateCw));
    assert_eq!(key_to_action("space"), Some(GameAction::HardDrop));
    assert_eq!(key_to_action("c"), Some(GameAction::Hold));
}

#[test]
fn key_to_action_ignores_unknown_keys() {
    assert_eq!(key_to_action("z"), None);
    assert_eq!(key_to_action("enter"), None);
}
