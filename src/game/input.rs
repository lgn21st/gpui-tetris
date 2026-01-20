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

#[derive(Clone, Copy, Debug)]
pub struct RepeatConfig {
    pub das_ms: u64,
    pub arr_ms: u64,
}

impl Default for RepeatConfig {
    fn default() -> Self {
        Self {
            das_ms: 150,
            arr_ms: 50,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RepeatState {
    held: bool,
    time_since_press_ms: u64,
    repeats_fired: u64,
}

impl RepeatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn press(&mut self) -> bool {
        if self.held {
            return false;
        }
        self.held = true;
        self.time_since_press_ms = 0;
        self.repeats_fired = 0;
        true
    }

    pub fn release(&mut self) {
        self.held = false;
        self.time_since_press_ms = 0;
        self.repeats_fired = 0;
    }

    pub fn tick(&mut self, elapsed_ms: u64, config: &RepeatConfig) -> u32 {
        if !self.held || config.arr_ms == 0 {
            return 0;
        }

        self.time_since_press_ms = self.time_since_press_ms.saturating_add(elapsed_ms);
        if self.time_since_press_ms < config.das_ms {
            return 0;
        }

        let total = (self.time_since_press_ms - config.das_ms) / config.arr_ms;
        let fired = total.saturating_sub(self.repeats_fired);
        self.repeats_fired = total;
        fired as u32
    }

    pub fn is_held(&self) -> bool {
        self.held
    }
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
