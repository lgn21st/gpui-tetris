use gpui::{Rgba, rgb};

use gpui_tetris::game::pieces::TetrominoType;

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const CELL_SIZE: f32 = 24.0;
pub const BOARD_WIDTH: f32 = 240.0;
pub const BOARD_HEIGHT: f32 = 480.0;
pub const BASE_PADDING: f32 = 16.0;
pub const BASE_GAP: f32 = 16.0;
pub const PANEL_WIDTH: f32 = 192.0;
pub const CONTENT_WIDTH: f32 = 448.0;
pub const PANEL_PADDING: f32 = 12.0;
pub const PANEL_SECTION_SPACING: f32 = 9.6;
pub const PANEL_ITEM_SPACING: f32 = 3.2;
pub const PANEL_DIVIDER_HEIGHT: f32 = 1.0;
pub const PANEL_DIVIDER_PADDING: f32 = 6.0;
pub const PREVIEW_CELL: f32 = 12.0;
pub const NEXT_PREVIEW_CELL: f32 = 10.0;
pub const PREVIEW_GAP: f32 = 2.0;
pub const PREVIEW_PADDING: f32 = 4.0;
pub const GROUP_CORNER_RADIUS: f32 = 12.0;
pub const PREVIEW_CORNER_RADIUS: f32 = 6.0;
pub const DEFAULT_SFX_VOLUME: f32 = 0.7;
pub const SFX_VOLUME_STEP: f32 = 0.1;
pub const MIN_SCALE: f32 = 0.6;
pub const BASE_PANEL_TEXT: f32 = 14.0;
pub const BASE_PANEL_SECTION_TEXT: f32 = 13.0;
pub const BASE_TITLE_TEXT: f32 = 24.0;
pub const BASE_HINT_TEXT: f32 = 14.0;
pub const BASE_ONBOARDING_TEXT: f32 = 12.0;
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
