use crate::game::board::{Board, BOARD_HEIGHT};
use crate::game::input::GameAction;
use crate::game::pieces::{spawn_position, Rotation, Tetromino, TetrominoType};

const NEXT_QUEUE_SIZE: usize = 5;

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

#[derive(Clone, Copy, Debug)]
pub struct GameConfig {
    pub tick_ms: u64,
    pub soft_drop_multiplier: u64,
    pub lock_delay_ms: u64,
    pub lock_reset_limit: u32,
    pub base_drop_ms: u64,
    pub soft_drop_grace_ms: u64,
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
        }
    }
}

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
    sound_events: Vec<SoundEvent>,
    rng: SimpleRng,
}

impl GameState {
    pub fn new(seed: u64, config: GameConfig) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut next_queue = Vec::new();
        refill_bag(&mut rng, &mut next_queue);
        ensure_queue(&mut rng, &mut next_queue);

        let first_kind = next_queue.remove(0);
        let (spawn_x, spawn_y) = spawn_position();
        let active = Tetromino::new(first_kind, spawn_x, spawn_y);

        Self {
            board: Board::new(),
            active,
            hold: None,
            can_hold: true,
            next_queue,
            score: 0,
            level: 0,
            lines: 0,
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
            sound_events: Vec::new(),
            rng,
        }
    }

    pub fn spawn_next(&mut self) {
        ensure_queue(&mut self.rng, &mut self.next_queue);

        let kind = self.next_queue.remove(0);
        let (spawn_x, spawn_y) = spawn_position();
        self.active = Tetromino::new(kind, spawn_x, spawn_y);
        self.active.rotation = Rotation::North;
        self.can_hold = true;
        self.lock_reset_count = 0;

        if !self.board.can_place(&self.active, self.active.x, self.active.y, self.active.rotation) {
            self.game_over = true;
            self.sound_events.push(SoundEvent::GameOver);
        }
    }

    pub fn apply_line_clear(&mut self, cleared: usize) {
        if cleared == 0 {
            return;
        }

        self.line_clear_timer_ms = 180;
        self.sound_events.push(SoundEvent::LineClear(cleared as u8));
        self.lines += cleared as u32;
        let level = self.level + 1;
        let points = match cleared {
            1 => 40,
            2 => 100,
            3 => 300,
            4 => 1200,
            _ => 0,
        };
        self.score += points * level;

        // Classic progression: advance level every 10 lines.
        self.level = self.lines / 10;
    }

    pub fn is_lock_row(&self) -> bool {
        self.active.y >= BOARD_HEIGHT as i32 - 1
    }

    pub fn drop_interval_ms(&self, soft_drop: bool) -> u64 {
        let mut interval = match self.level {
            0 => 1000,
            1 => 800,
            2 => 650,
            3 => 500,
            4 => 400,
            5 => 320,
            6 => 250,
            7 => 200,
            8 => 160,
            _ => 120,
        };
        interval = interval.min(self.base_drop_ms).max(100);
        if soft_drop {
            let adjusted = interval / self.soft_drop_multiplier.max(1);
            return adjusted.max(1);
        }
        interval
    }

    pub fn tick(&mut self, elapsed_ms: u64, soft_drop: bool) {
        if self.game_over || self.paused {
            return;
        }

        if self.line_clear_timer_ms > 0 {
            self.line_clear_timer_ms = self.line_clear_timer_ms.saturating_sub(elapsed_ms);
            if self.line_clear_timer_ms > 0 {
                return;
            }
        }

        self.drop_timer_ms = self.drop_timer_ms.saturating_add(elapsed_ms);
        if self.soft_drop_timeout_ms > 0 {
            self.soft_drop_timeout_ms = self.soft_drop_timeout_ms.saturating_sub(elapsed_ms);
            if self.soft_drop_timeout_ms == 0 {
                self.soft_drop_active = false;
            }
        }
        let interval = self.drop_interval_ms(soft_drop || self.soft_drop_active);

        while self.drop_timer_ms >= interval {
            self.drop_timer_ms -= interval;
            let _ = self.try_move(0, 1);
        }

        if self.can_move_down() {
            self.lock_timer_ms = 0;
            self.lock_reset_count = 0;
        } else {
            self.lock_timer_ms = self.lock_timer_ms.saturating_add(elapsed_ms);
            if self.lock_timer_ms >= self.lock_delay_ms {
                self.lock_timer_ms = 0;
                self.drop_timer_ms = 0;
                self.board.lock_piece(&self.active);
                let cleared = self.board.clear_lines();
                self.apply_line_clear(cleared);
                self.spawn_next();
            }
        }
    }

    pub fn apply_action(&mut self, action: GameAction) {
        if self.game_over && action != GameAction::Restart {
            return;
        }
        if self.paused && action != GameAction::Pause && action != GameAction::Restart {
            return;
        }

        match action {
            GameAction::MoveLeft => {
                self.try_move(-1, 0);
                self.sound_events.push(SoundEvent::Move);
            }
            GameAction::MoveRight => {
                self.try_move(1, 0);
                self.sound_events.push(SoundEvent::Move);
            }
            GameAction::SoftDrop => {
                if self.try_move(0, 1) {
                    self.score = self.score.saturating_add(1);
                }
                self.activate_soft_drop();
                self.sound_events.push(SoundEvent::SoftDrop);
            }
            GameAction::HardDrop => {
                let mut dropped = 0;
                while self.try_move(0, 1) {
                    dropped += 1;
                }
                if dropped > 0 {
                    self.score = self.score.saturating_add(dropped * 2);
                }
                self.sound_events.push(SoundEvent::HardDrop);
                self.board.lock_piece(&self.active);
                let cleared = self.board.clear_lines();
                self.apply_line_clear(cleared);
                self.spawn_next();
                self.lock_timer_ms = 0;
                self.drop_timer_ms = 0;
            }
            GameAction::RotateCw => {
                self.try_rotate(true);
                self.sound_events.push(SoundEvent::Rotate);
            }
            GameAction::RotateCcw => {
                self.try_rotate(false);
                self.sound_events.push(SoundEvent::Rotate);
            }
            GameAction::Hold => {
                if !self.can_hold {
                    return;
                }

                let current_kind = self.active.kind;
                if let Some(held_kind) = self.hold {
                    self.hold = Some(current_kind);
                    self.active = self.spawn_piece(held_kind);
                } else {
                    self.hold = Some(current_kind);
                    self.spawn_next();
                }
                self.can_hold = false;
                self.sound_events.push(SoundEvent::Hold);
            }
            GameAction::Pause => {
                self.paused = !self.paused;
            }
            GameAction::Restart => {
                self.reset();
            }
        }
    }

    fn spawn_piece(&mut self, kind: TetrominoType) -> Tetromino {
        let (spawn_x, spawn_y) = spawn_position();
        let piece = Tetromino::new(kind, spawn_x, spawn_y);
        if !self
            .board
            .can_place(&piece, piece.x, piece.y, piece.rotation)
        {
            self.game_over = true;
            self.sound_events.push(SoundEvent::GameOver);
        }
        piece
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

    pub fn reset(&mut self) {
        let seed = self.rng.next_u32() as u64;
        let config = GameConfig {
            tick_ms: self.tick_ms,
            soft_drop_multiplier: self.soft_drop_multiplier,
            lock_delay_ms: self.lock_delay_ms,
            lock_reset_limit: self.lock_reset_limit,
            base_drop_ms: self.base_drop_ms,
            soft_drop_grace_ms: self.soft_drop_grace_ms,
        };
        *self = GameState::new(seed, config);
    }

    pub fn activate_soft_drop(&mut self) {
        self.soft_drop_active = true;
        self.soft_drop_timeout_ms = self.soft_drop_grace_ms;
    }

    pub fn is_soft_drop_active(&self) -> bool {
        self.soft_drop_active
    }

    pub fn ghost_blocks(&self) -> [(i32, i32); 4] {
        let mut ghost_y = self.active.y;
        while self
            .board
            .can_place(&self.active, self.active.x, ghost_y + 1, self.active.rotation)
        {
            ghost_y += 1;
        }

        let mut blocks = self.active.blocks(self.active.rotation);
        for (x, y) in blocks.iter_mut() {
            *x += self.active.x;
            *y += ghost_y;
        }
        blocks
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let new_x = self.active.x + dx;
        let new_y = self.active.y + dy;
        if self
            .board
            .can_place(&self.active, new_x, new_y, self.active.rotation)
        {
            self.active.x = new_x;
            self.active.y = new_y;
            self.handle_lock_reset();
            return true;
        }
        false
    }

    fn try_rotate(&mut self, clockwise: bool) -> bool {
        let next_rotation = if clockwise {
            self.active.rotation.cw()
        } else {
            self.active.rotation.ccw()
        };

        let kicks = [
            (0, 0),
            (-1, 0),
            (1, 0),
            (-2, 0),
            (2, 0),
            (0, -1),
            (0, -2),
            (-1, -1),
            (1, -1),
            (0, 1),
        ];
        for (dx, dy) in kicks.iter() {
            let new_x = self.active.x + dx;
            let new_y = self.active.y + dy;
            if self
                .board
                .can_place(&self.active, new_x, new_y, next_rotation)
            {
                self.active.x = new_x;
                self.active.y = new_y;
                self.active.rotation = next_rotation;
                self.handle_lock_reset();
                return true;
            }
        }

        false
    }

    fn can_move_down(&self) -> bool {
        self.board
            .can_place(&self.active, self.active.x, self.active.y + 1, self.active.rotation)
    }

    fn handle_lock_reset(&mut self) {
        if self.can_move_down() {
            self.lock_timer_ms = 0;
            self.lock_reset_count = 0;
            return;
        }

        if self.lock_reset_count < self.lock_reset_limit {
            self.lock_timer_ms = 0;
            self.lock_reset_count += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // LCG constants from Numerical Recipes.
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 16) as u32
    }

    fn next_range(&mut self, upper: usize) -> usize {
        if upper == 0 {
            return 0;
        }
        (self.next_u32() as usize) % upper
    }
}

fn refill_bag(rng: &mut SimpleRng, queue: &mut Vec<TetrominoType>) {
    let mut bag = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];

    // Fisher-Yates shuffle.
    for i in (1..bag.len()).rev() {
        let j = rng.next_range(i + 1);
        bag.swap(i, j);
    }

    queue.extend_from_slice(&bag);
}

fn ensure_queue(rng: &mut SimpleRng, queue: &mut Vec<TetrominoType>) {
    while queue.len() < NEXT_QUEUE_SIZE {
        refill_bag(rng, queue);
    }
}
