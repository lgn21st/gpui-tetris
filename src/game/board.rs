use crate::game::pieces::{Rotation, Tetromino};

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub filled: bool,
    pub kind: Option<crate::game::pieces::TetrominoType>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            filled: false,
            kind: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    pub cells: [[Cell; BOARD_WIDTH]; BOARD_HEIGHT],
}

impl Board {
    pub fn new() -> Self {
        Self {
            cells: [[Cell::default(); BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }

    pub fn is_inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < BOARD_WIDTH as i32 && y >= 0 && y < BOARD_HEIGHT as i32
    }

    pub fn is_occupied(&self, x: i32, y: i32) -> bool {
        if !self.is_inside(x, y) {
            return true;
        }
        self.cells[y as usize][x as usize].filled
    }

    pub fn can_place(&self, piece: &Tetromino, x: i32, y: i32, rotation: Rotation) -> bool {
        for (dx, dy) in piece.blocks(rotation) {
            let nx = x + dx;
            let ny = y + dy;
            if self.is_occupied(nx, ny) {
                return false;
            }
        }
        true
    }

    pub fn lock_piece(&mut self, piece: &Tetromino) {
        for (dx, dy) in piece.blocks(piece.rotation) {
            let nx = piece.x + dx;
            let ny = piece.y + dy;
            if self.is_inside(nx, ny) {
                let cell = &mut self.cells[ny as usize][nx as usize];
                cell.filled = true;
                cell.kind = Some(piece.kind);
            }
        }
    }

    pub fn clear_lines(&mut self) -> usize {
        let mut cleared = 0;
        let mut write_row = BOARD_HEIGHT as i32 - 1;

        for read_row in (0..BOARD_HEIGHT as i32).rev() {
            let full = (0..BOARD_WIDTH as i32)
                .all(|x| self.cells[read_row as usize][x as usize].filled);

            if full {
                cleared += 1;
            } else {
                if write_row != read_row {
                    self.cells[write_row as usize] = self.cells[read_row as usize];
                }
                write_row -= 1;
            }
        }

        for y in 0..=write_row {
            self.cells[y as usize] = [Cell::default(); BOARD_WIDTH];
        }

        cleared
    }
}
