use gpui::{Rgba, rgb};

use gpui_tetris::game::pieces::TetrominoType;

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const CELL_SIZE: f32 = 24.0;

pub const BASE_WINDOW_WIDTH: f32 = WINDOW_WIDTH;
pub const BASE_WINDOW_HEIGHT: f32 = WINDOW_HEIGHT;
pub const BASE_CELL_SIZE: f32 = CELL_SIZE;

pub const BOARD_COLS: f32 = 10.0;
pub const BOARD_ROWS: f32 = 20.0;
pub const BASE_PADDING: f32 = 16.0;
pub const BASE_GAP: f32 = 16.0;
pub const DEFAULT_SFX_VOLUME: f32 = 0.7;
pub const SFX_VOLUME_STEP: f32 = 0.1;
pub const MIN_SCALE: f32 = 0.6;
pub const BASE_PANEL_TEXT: f32 = 12.0;
pub const BASE_TITLE_TEXT: f32 = 24.0;
pub const BASE_HINT_TEXT: f32 = 14.0;
pub const CONTROLLER_AXIS_THRESHOLD: f32 = 0.5;

pub fn piece_color(kind: TetrominoType) -> Rgba {
    match kind {
        TetrominoType::I => rgb(0x4fd1c5),
        TetrominoType::O => rgb(0xf6e05e),
        TetrominoType::T => rgb(0x9f7aea),
        TetrominoType::S => rgb(0x68d391),
        TetrominoType::Z => rgb(0xfc8181),
        TetrominoType::J => rgb(0x63b3ed),
        TetrominoType::L => rgb(0xf6ad55),
    }
}
