use crate::game::board::{Board, BOARD_HEIGHT};
use crate::game::input::GameAction;
use crate::game::pieces::{spawn_position, Rotation, Tetromino, TetrominoType};

#[derive(Clone, Copy, Debug)]
pub struct GameConfig {
    pub tick_ms: u64,
    pub soft_drop_multiplier: u64,
    pub lock_delay_ms: u64,
    pub base_drop_ms: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            tick_ms: 16,
            soft_drop_multiplier: 10,
            lock_delay_ms: 500,
            base_drop_ms: 1000,
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
    pub tick_ms: u64,
    pub soft_drop_multiplier: u64,
    pub lock_delay_ms: u64,
    pub base_drop_ms: u64,
    pub drop_timer_ms: u64,
    pub lock_timer_ms: u64,
    rng: SimpleRng,
}

impl GameState {
    pub fn new(seed: u64, config: GameConfig) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut next_queue = Vec::new();
        refill_bag(&mut rng, &mut next_queue);

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
            tick_ms: config.tick_ms,
            soft_drop_multiplier: config.soft_drop_multiplier,
            lock_delay_ms: config.lock_delay_ms,
            base_drop_ms: config.base_drop_ms,
            drop_timer_ms: 0,
            lock_timer_ms: 0,
            rng,
        }
    }

    pub fn spawn_next(&mut self) {
        if self.next_queue.is_empty() {
            refill_bag(&mut self.rng, &mut self.next_queue);
        }

        let kind = self.next_queue.remove(0);
        let (spawn_x, spawn_y) = spawn_position();
        self.active = Tetromino::new(kind, spawn_x, spawn_y);
        self.active.rotation = Rotation::North;
        self.can_hold = true;

        if !self.board.can_place(&self.active, self.active.x, self.active.y, self.active.rotation) {
            self.game_over = true;
        }
    }

    pub fn apply_line_clear(&mut self, cleared: usize) {
        if cleared == 0 {
            return;
        }

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
        let level_factor = self.level as u64 * 50;
        let mut interval = self.base_drop_ms.saturating_sub(level_factor);
        if interval < 100 {
            interval = 100;
        }
        if soft_drop {
            let adjusted = interval / self.soft_drop_multiplier.max(1);
            return adjusted.max(1);
        }
        interval
    }

    pub fn tick(&mut self, elapsed_ms: u64, soft_drop: bool) {
        if self.game_over {
            return;
        }

        self.drop_timer_ms = self.drop_timer_ms.saturating_add(elapsed_ms);
        let interval = self.drop_interval_ms(soft_drop);

        while self.drop_timer_ms >= interval {
            self.drop_timer_ms -= interval;
            let _ = self.try_move(0, 1);
        }

        if self.can_move_down() {
            self.lock_timer_ms = 0;
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
        if self.game_over {
            return;
        }

        match action {
            GameAction::MoveLeft => {
                self.try_move(-1, 0);
            }
            GameAction::MoveRight => {
                self.try_move(1, 0);
            }
            GameAction::SoftDrop => {
                self.try_move(0, 1);
            }
            GameAction::HardDrop => {
                while self.try_move(0, 1) {}
                self.board.lock_piece(&self.active);
                let cleared = self.board.clear_lines();
                self.apply_line_clear(cleared);
                self.spawn_next();
                self.lock_timer_ms = 0;
                self.drop_timer_ms = 0;
            }
            GameAction::RotateCw => {
                self.try_rotate(true);
            }
            GameAction::RotateCcw => {
                self.try_rotate(false);
            }
            GameAction::Hold | GameAction::Pause | GameAction::Restart => {}
        }
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

        if self
            .board
            .can_place(&self.active, self.active.x, self.active.y, next_rotation)
        {
            self.active.rotation = next_rotation;
            return true;
        }

        false
    }

    fn can_move_down(&self) -> bool {
        self.board
            .can_place(&self.active, self.active.x, self.active.y + 1, self.active.rotation)
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
