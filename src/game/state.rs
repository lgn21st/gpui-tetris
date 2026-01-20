use crate::game::board::{BOARD_HEIGHT, Board};
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType, spawn_position};

mod actions;
mod kicks;
mod rng;
mod scoring;
mod timing;
mod types;

use actions::{
    activate_soft_drop, apply_action, can_move_down, ghost_blocks, lock_active_piece, try_move,
};
use rng::{SimpleRng, ensure_queue, refill_bag};
use scoring::apply_line_clear;
use timing::{drop_interval_ms, tick};
pub use types::{GameConfig, RulesConfig, Ruleset, SoundEvent, TSpinKind};

const NEXT_QUEUE_SIZE: usize = 5;

#[derive(Clone, Debug)]
pub struct GameState {
    pub board: Board,
    pub active: Tetromino,
    pub hold: Option<TetrominoType>,
    pub can_hold: bool,
    pub next_queue: Vec<TetrominoType>,
    pub score: u32,
    pub level: u32,
    pub lines: u32,
    pub combo: i32,
    pub back_to_back: bool,
    pub ruleset: Ruleset,
    pub rules: types::RulesConfig,
    pub game_over: bool,
    pub paused: bool,
    pub tick_ms: u64,
    pub soft_drop_multiplier: u64,
    pub lock_delay_ms: u64,
    pub lock_reset_limit: u32,
    pub lock_reset_count: u32,
    pub base_drop_ms: u64,
    pub soft_drop_grace_ms: u64,
    pub soft_drop_active: bool,
    pub soft_drop_timeout_ms: u64,
    pub drop_timer_ms: u64,
    pub lock_timer_ms: u64,
    pub line_clear_timer_ms: u64,
    pub landing_flash_timer_ms: u64,
    pub last_lock_cells: [(i32, i32); 4],
    pub ghost_cache: [(i32, i32); 4],
    pub board_revision: u64,
    sound_events: Vec<SoundEvent>,
    last_action_rotate: bool,
    rng: SimpleRng,
}

impl GameState {
    pub fn new(seed: u64, config: GameConfig) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut next_queue = init_next_queue(&mut rng);
        let active = spawn_first_piece(&mut next_queue);
        let mut state = Self {
            board: Board::new(),
            active,
            hold: None,
            can_hold: true,
            next_queue,
            score: 0,
            level: 0,
            lines: 0,
            combo: -1,
            back_to_back: false,
            ruleset: config.ruleset,
            rules: config.rules,
            game_over: false,
            paused: false,
            tick_ms: config.tick_ms,
            soft_drop_multiplier: config.soft_drop_multiplier,
            lock_delay_ms: config.lock_delay_ms,
            lock_reset_limit: config.lock_reset_limit,
            lock_reset_count: 0,
            base_drop_ms: config.base_drop_ms,
            soft_drop_grace_ms: config.soft_drop_grace_ms,
            soft_drop_active: false,
            soft_drop_timeout_ms: 0,
            drop_timer_ms: 0,
            lock_timer_ms: 0,
            line_clear_timer_ms: 0,
            landing_flash_timer_ms: 0,
            last_lock_cells: [(0, 0); 4],
            ghost_cache: [(0, 0); 4],
            board_revision: 0,
            sound_events: Vec::new(),
            last_action_rotate: false,
            rng,
        };
        actions::update_ghost_cache(&mut state);
        state
    }

    pub fn spawn_next(&mut self) {
        ensure_queue(&mut self.rng, &mut self.next_queue);

        let kind = self.next_queue.remove(0);
        let (spawn_x, spawn_y) = spawn_position();
        self.active = Tetromino::new(kind, spawn_x, spawn_y);
        self.active.rotation = Rotation::North;
        self.can_hold = true;
        self.lock_reset_count = 0;
        self.last_action_rotate = false;
        actions::update_ghost_cache(self);

        if !self.board.can_place(
            &self.active,
            self.active.x,
            self.active.y,
            self.active.rotation,
        ) {
            self.game_over = true;
            self.sound_events.push(SoundEvent::GameOver);
        }
    }

    pub fn apply_line_clear(&mut self, cleared: usize, t_spin: TSpinKind) {
        apply_line_clear(self, cleared, t_spin);
    }

    pub fn is_lock_row(&self) -> bool {
        self.active.y >= BOARD_HEIGHT as i32 - 1
    }

    pub fn drop_interval_ms(&self, soft_drop: bool) -> u64 {
        drop_interval_ms(self, soft_drop)
    }

    pub fn tick(&mut self, elapsed_ms: u64, soft_drop: bool) {
        tick(self, elapsed_ms, soft_drop);
    }

    pub fn apply_action(&mut self, action: GameAction) {
        apply_action(self, action);
    }

    pub fn take_sound_events(&mut self) -> Vec<SoundEvent> {
        std::mem::take(&mut self.sound_events)
    }

    pub fn is_line_clear_active(&self) -> bool {
        self.line_clear_timer_ms > 0
    }

    pub fn lock_reset_remaining(&self) -> u32 {
        self.lock_reset_limit.saturating_sub(self.lock_reset_count)
    }

    pub fn is_grounded(&self) -> bool {
        !self.can_move_down()
    }

    pub fn is_classic_ruleset(&self) -> bool {
        self.ruleset == Ruleset::Classic
    }

    pub fn lock_warning_active(&self) -> bool {
        if self.lock_delay_ms == 0 {
            return false;
        }
        self.is_grounded() && self.lock_timer_ms >= (self.lock_delay_ms * 3 / 5)
    }

    pub fn lock_warning_intensity(&self) -> f32 {
        if !self.lock_warning_active() {
            return 0.0;
        }
        if (self.lock_timer_ms / 120) % 2 == 0 {
            0.12
        } else {
            0.22
        }
    }

    pub fn landing_flash_active(&self) -> bool {
        self.landing_flash_timer_ms > 0
    }

    pub fn reset(&mut self) {
        let seed = self.rng.next_u32() as u64;
        *self = GameState::new(seed, self.current_config());
    }

    pub fn board_revision(&self) -> u64 {
        self.board_revision
    }

    pub fn activate_soft_drop(&mut self) {
        activate_soft_drop(self);
    }

    pub fn is_soft_drop_active(&self) -> bool {
        self.soft_drop_active
    }

    fn current_config(&self) -> GameConfig {
        GameConfig {
            tick_ms: self.tick_ms,
            soft_drop_multiplier: self.soft_drop_multiplier,
            lock_delay_ms: self.lock_delay_ms,
            lock_reset_limit: self.lock_reset_limit,
            base_drop_ms: self.base_drop_ms,
            soft_drop_grace_ms: self.soft_drop_grace_ms,
            ruleset: self.ruleset,
            rules: self.rules,
        }
    }

    pub fn ghost_blocks(&self) -> [(i32, i32); 4] {
        ghost_blocks(self)
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        try_move(self, dx, dy)
    }

    fn can_move_down(&self) -> bool {
        can_move_down(self)
    }

    fn lock_active_piece(&mut self) {
        lock_active_piece(self);
    }
}

fn init_next_queue(rng: &mut SimpleRng) -> Vec<TetrominoType> {
    let mut next_queue = Vec::new();
    refill_bag(rng, &mut next_queue);
    ensure_queue(rng, &mut next_queue);
    next_queue
}

fn spawn_first_piece(next_queue: &mut Vec<TetrominoType>) -> Tetromino {
    let first_kind = next_queue.remove(0);
    let (spawn_x, spawn_y) = spawn_position();
    Tetromino::new(first_kind, spawn_x, spawn_y)
}
