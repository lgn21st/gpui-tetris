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
