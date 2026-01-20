use crate::game::pieces::TetrominoType;

#[derive(Clone, Debug)]
pub(super) struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub(super) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(super) fn next_u32(&mut self) -> u32 {
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

pub(super) fn refill_bag(rng: &mut SimpleRng, queue: &mut Vec<TetrominoType>) {
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

pub(super) fn ensure_queue(rng: &mut SimpleRng, queue: &mut Vec<TetrominoType>) {
    while queue.len() < super::NEXT_QUEUE_SIZE {
        refill_bag(rng, queue);
    }
}
