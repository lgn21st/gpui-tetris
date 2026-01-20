#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEvent {
    Move,
    Rotate,
    SoftDrop,
    HardDrop,
    LineClear(u8),
    GameOver,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ruleset {
    Classic,
    Modern,
}

#[derive(Clone, Copy, Debug)]
pub struct GameConfig {
    pub tick_ms: u64,
    pub soft_drop_multiplier: u64,
    pub lock_delay_ms: u64,
    pub lock_reset_limit: u32,
    pub base_drop_ms: u64,
    pub soft_drop_grace_ms: u64,
    pub ruleset: Ruleset,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            tick_ms: 16,
            soft_drop_multiplier: 10,
            lock_delay_ms: 450,
            lock_reset_limit: 15,
            base_drop_ms: 1000,
            soft_drop_grace_ms: 150,
            ruleset: Ruleset::Classic,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TSpinKind {
    None,
    Mini,
    Full,
}
