use crate::game::board::Board;
use crate::game::input::GameAction;
use crate::game::pieces::{Rotation, Tetromino, TetrominoType, spawn_position};

mod actions;
pub(crate) mod kicks;
mod rng;
mod scoring;
mod timing;
mod types;

#[cfg(test)]
mod tests;

use actions::{apply_action, can_move_down, ghost_blocks, ghost_y, lock_active_piece, try_move};
use rng::{SimpleRng, ensure_queue};
use scoring::apply_line_clear;
use timing::{drop_interval_ms, tick};
pub use types::{GameConfig, GameEvent, RulesConfig, Ruleset, SoundEvent, TSpinKind};

const NEXT_QUEUE_PREVIEW_SIZE: usize = 5;
const SPAWN_QUEUE_MIN: usize = 6;
const MAX_PROTOCOL_EVENTS: usize = 4;
const MAX_SOUND_EVENTS: usize = 256;

/// Authoritative state. Configure a new game with `GameConfig`; advance it with
/// actions, ticks or seeded resets. Readers cannot mutate fields or board cells.
///
/// ```compile_fail
/// use gpui_tetris::game::state::GameState;
/// let mut game = GameState::new(1, Default::default());
/// game.paused = true;
/// ```
///
/// ```compile_fail
/// use gpui_tetris::game::state::GameState;
/// let mut game = GameState::new(1, Default::default());
/// game.board().cells[0][0].kind = None;
/// ```
#[derive(Clone, Debug)]
pub struct GameState {
    board: Board,
    active: Tetromino,
    hold: Option<TetrominoType>,
    can_hold: bool,
    next_queue: Vec<TetrominoType>,
    score: u32,
    level: u32,
    lines: u32,
    combo: i32,
    back_to_back: bool,
    ruleset: Ruleset,
    rules: types::RulesConfig,
    game_over: bool,
    paused: bool,
    tick_ms: u64,
    soft_drop_multiplier: u64,
    lock_delay_ms: u64,
    lock_reset_limit: u32,
    lock_reset_count: u32,
    base_drop_ms: u64,
    soft_drop_grace_ms: u64,
    soft_drop_active: bool,
    soft_drop_timeout_ms: u64,
    drop_timer_ms: u64,
    lock_timer_ms: u64,
    line_clear_timer_ms: u64,
    landing_flash_timer_ms: u64,
    last_lock_cells: [(i32, i32); 4],
    ghost_cache: [(i32, i32); 4],
    active_moved_since_spawn: bool,
    board_revision: u64,
    seed: u64,
    episode_id: u64,
    piece_id: u64,
    step_in_piece: u64,
    logical_step: u64,
    sound_events: Vec<SoundEvent>,
    protocol_events: Vec<GameEvent>,
    last_action_rotate: bool,
    rng: SimpleRng,
}

impl GameState {
    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn active(&self) -> Tetromino {
        self.active
    }

    pub fn hold(&self) -> Option<TetrominoType> {
        self.hold
    }

    pub fn can_hold(&self) -> bool {
        self.can_hold
    }

    pub fn next_queue(&self) -> &[TetrominoType] {
        &self.next_queue
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn lines(&self) -> u32 {
        self.lines
    }

    pub fn combo(&self) -> i32 {
        self.combo
    }

    pub fn back_to_back(&self) -> bool {
        self.back_to_back
    }

    pub fn game_over(&self) -> bool {
        self.game_over
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn tick_ms(&self) -> u64 {
        self.tick_ms
    }

    pub fn lock_delay_ms(&self) -> u64 {
        self.lock_delay_ms
    }

    pub fn lock_reset_count(&self) -> u32 {
        self.lock_reset_count
    }

    pub fn soft_drop_timeout_ms(&self) -> u64 {
        self.soft_drop_timeout_ms
    }

    pub fn drop_timer_ms(&self) -> u64 {
        self.drop_timer_ms
    }

    pub fn lock_timer_ms(&self) -> u64 {
        self.lock_timer_ms
    }

    pub fn line_clear_timer_ms(&self) -> u64 {
        self.line_clear_timer_ms
    }

    pub fn last_lock_cells(&self) -> [(i32, i32); 4] {
        self.last_lock_cells
    }

    pub fn active_moved_since_spawn(&self) -> bool {
        self.active_moved_since_spawn
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn episode_id(&self) -> u64 {
        self.episode_id
    }

    pub fn piece_id(&self) -> u64 {
        self.piece_id
    }

    pub fn step_in_piece(&self) -> u64 {
        self.step_in_piece
    }

    pub fn logical_step(&self) -> u64 {
        self.logical_step
    }

    pub fn new(seed: u64, config: GameConfig) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut next_queue = init_next_queue(&mut rng);
        let active = spawn_first_piece(&mut next_queue);
        ensure_queue(&mut rng, &mut next_queue, NEXT_QUEUE_PREVIEW_SIZE);
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
            active_moved_since_spawn: false,
            board_revision: 1,
            seed,
            episode_id: 0,
            piece_id: 0,
            step_in_piece: 0,
            logical_step: 0,
            sound_events: Vec::new(),
            protocol_events: Vec::with_capacity(MAX_PROTOCOL_EVENTS),
            last_action_rotate: false,
            rng,
        };
        actions::update_ghost_cache(&mut state);
        state
    }

    fn spawn_next(&mut self) {
        ensure_queue(&mut self.rng, &mut self.next_queue, SPAWN_QUEUE_MIN);

        let kind = self.next_queue.remove(0);
        let (spawn_x, spawn_y) = spawn_position();
        self.active = Tetromino::new(kind, spawn_x, spawn_y);
        self.active.rotation = Rotation::North;
        self.can_hold = true;
        self.lock_reset_count = 0;
        self.last_action_rotate = false;
        self.active_moved_since_spawn = false;
        self.piece_id = self.piece_id.saturating_add(1);
        self.step_in_piece = 0;
        actions::update_ghost_cache(self);

        if !self.board.can_place(
            &self.active,
            self.active.x,
            self.active.y,
            self.active.rotation,
        ) {
            self.game_over = true;
            self.push_sound_event(SoundEvent::GameOver);
        }
    }

    fn apply_line_clear(&mut self, cleared: usize, t_spin: TSpinKind) {
        apply_line_clear(self, cleared, t_spin);
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

    pub fn drain_sound_events(&mut self) -> impl Iterator<Item = SoundEvent> + '_ {
        self.sound_events.drain(..)
    }

    pub(crate) fn push_sound_event(&mut self, event: SoundEvent) {
        push_bounded(&mut self.sound_events, event, MAX_SOUND_EVENTS);
    }

    /// Events and identity are borrowed from the same latest logical transition.
    pub fn transition(&self) -> Transition<'_> {
        Transition {
            logical_step: self.logical_step,
            events: &self.protocol_events,
        }
    }

    pub(crate) fn push_protocol_event(&mut self, event: GameEvent) {
        push_bounded(&mut self.protocol_events, event, MAX_PROTOCOL_EVENTS);
    }

    pub(crate) fn advance_logical_step(&mut self) {
        self.protocol_events.clear();
        self.logical_step = self.logical_step.saturating_add(1);
        self.step_in_piece = self.step_in_piece.saturating_add(1);
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
        self.is_grounded() && self.lock_timer_ms >= self.lock_delay_ms.saturating_mul(3) / 5
    }

    pub fn lock_warning_intensity(&self) -> f32 {
        if !self.lock_warning_active() {
            return 0.0;
        }
        if (self.lock_timer_ms / 120).is_multiple_of(2) {
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
        self.reset_with_seed(seed);
    }

    pub fn reset_with_seed(&mut self, seed: u64) {
        let next_episode = self.episode_id.saturating_add(1);
        let next_step = self.logical_step.saturating_add(1);
        let board_changed = self
            .board
            .cells
            .iter()
            .flatten()
            .any(|cell| cell.kind.is_some());
        let next_board_revision = self.board_revision.wrapping_add(u64::from(board_changed));
        let mut next = GameState::new(seed, self.current_config());
        next.episode_id = next_episode;
        next.logical_step = next_step;
        next.board_revision = next_board_revision;
        *self = next;
    }

    pub fn board_revision(&self) -> u64 {
        self.board_revision
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

    pub fn ghost_y(&self) -> i32 {
        ghost_y(self)
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

fn push_bounded<T>(items: &mut Vec<T>, item: T, capacity: usize) {
    if items.len() >= capacity {
        items.remove(0);
    }
    items.push(item);
}

fn init_next_queue(rng: &mut SimpleRng) -> Vec<TetrominoType> {
    let mut next_queue = Vec::new();
    ensure_queue(rng, &mut next_queue, SPAWN_QUEUE_MIN);
    next_queue
}

fn spawn_first_piece(next_queue: &mut Vec<TetrominoType>) -> Tetromino {
    let first_kind = next_queue.remove(0);
    let (spawn_x, spawn_y) = spawn_position();
    Tetromino::new(first_kind, spawn_x, spawn_y)
}

#[derive(Clone, Copy, Debug)]
pub struct Transition<'a> {
    pub logical_step: u64,
    pub events: &'a [GameEvent],
}
