#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameAction {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCw,
    RotateCcw,
    Hold,
    Pause,
    Restart,
}

pub fn key_to_action(key: &str) -> Option<GameAction> {
    match key {
        "left" => Some(GameAction::MoveLeft),
        "right" => Some(GameAction::MoveRight),
        "down" => Some(GameAction::SoftDrop),
        "up" => Some(GameAction::RotateCw),
        "space" => Some(GameAction::HardDrop),
        "c" => Some(GameAction::Hold),
        _ => None,
    }
}
