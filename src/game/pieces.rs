#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TetrominoType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

impl Rotation {
    pub fn cw(self) -> Self {
        match self {
            Rotation::North => Rotation::East,
            Rotation::East => Rotation::South,
            Rotation::South => Rotation::West,
            Rotation::West => Rotation::North,
        }
    }

    pub fn ccw(self) -> Self {
        match self {
            Rotation::North => Rotation::West,
            Rotation::West => Rotation::South,
            Rotation::South => Rotation::East,
            Rotation::East => Rotation::North,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Rotation::North => 0,
            Rotation::East => 1,
            Rotation::South => 2,
            Rotation::West => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tetromino {
    pub kind: TetrominoType,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
}

impl Tetromino {
    pub fn new(kind: TetrominoType, x: i32, y: i32) -> Self {
        Self {
            kind,
            rotation: Rotation::North,
            x,
            y,
        }
    }

    pub fn blocks(&self, rotation: Rotation) -> [(i32, i32); 4] {
        shape_for(self.kind, rotation)
    }
}

pub fn spawn_position() -> (i32, i32) {
    // Classic spawn near the top center of a 10x20 board.
    (3, 0)
}

fn shape_for(kind: TetrominoType, rotation: Rotation) -> [(i32, i32); 4] {
    const SHAPES: [[[(i32, i32); 4]; 4]; 7] = [
        // I
        [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ],
        // O
        [
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
        ],
        // T
        [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ],
        // S
        [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ],
        // Z
        [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ],
        // J
        [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ],
        // L
        [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ],
    ];

    let kind_index = match kind {
        TetrominoType::I => 0,
        TetrominoType::O => 1,
        TetrominoType::T => 2,
        TetrominoType::S => 3,
        TetrominoType::Z => 4,
        TetrominoType::J => 5,
        TetrominoType::L => 6,
    };

    SHAPES[kind_index][rotation.index()]
}
