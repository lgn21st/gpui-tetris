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
pub struct RulesConfig {
    pub classic_line_scores: [u32; 4],
    pub t_spin_full: [u32; 4],
    pub t_spin_mini: [u32; 3],
    pub combo_base: u32,
    pub b2b_bonus_num: u32,
    pub b2b_bonus_den: u32,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            classic_line_scores: [40, 100, 300, 1200],
            t_spin_full: [400, 800, 1200, 1600],
            t_spin_mini: [100, 200, 400],
            combo_base: 50,
            b2b_bonus_num: 3,
            b2b_bonus_den: 2,
        }
    }
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
    pub rules: RulesConfig,
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
            rules: RulesConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TSpinKind {
    None,
    Mini,
    Full,
}
