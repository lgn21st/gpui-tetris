use gpui_kit::{Rgba, rgb, rgba};

use crate::ui::style::piece_color;
use gpui_tetris::game::pieces::TetrominoType;

pub fn app_bg() -> Rgba {
    rgb(0x101010)
}

pub fn board_bg() -> Rgba {
    rgb(0x1c1c1c)
}

pub fn panel_bg() -> Rgba {
    rgb(0x1a1a1a)
}

pub fn border() -> Rgba {
    rgb(0x2e2e2e)
}

pub fn group_start() -> Rgba {
    rgb(0x181818)
}

pub fn group_end() -> Rgba {
    rgb(0x0c0c0c)
}

pub fn divider() -> Rgba {
    rgba(0x2e2e2e4d)
}

pub fn gridline() -> Rgba {
    rgba(0x36363659)
}

pub fn preview_bg() -> Rgba {
    rgba(0x00000073)
}

pub fn preview_gridline() -> Rgba {
    rgba(0xffffff26)
}

pub fn ghost_fill() -> Rgba {
    rgb(0x2a2a2a)
}

pub fn flash_border() -> Rgba {
    rgba(0xffffffe6)
}

pub fn overlay_bg() -> Rgba {
    rgb(0x000000)
}

pub fn overlay_text() -> Rgba {
    rgb(0xf5f5f5)
}

pub fn game_over_tint() -> Rgba {
    rgb(0x3a0f0f)
}

pub fn lock_bar_bg() -> Rgba {
    rgb(0x1f2937)
}

pub fn lock_bar_safe() -> Rgba {
    rgb(0x34d399)
}

pub fn lock_bar_danger() -> Rgba {
    rgb(0xf87171)
}

pub fn panel_text() -> Rgba {
    rgb(0xe6e6e6)
}

pub fn secondary_text() -> Rgba {
    rgb(0xa3a3a3)
}

pub fn piece_fill(kind: Option<TetrominoType>, ghost: bool) -> Rgba {
    match kind {
        Some(piece) => {
            if ghost {
                ghost_fill()
            } else {
                piece_color(piece)
            }
        }
        None => board_bg(),
    }
}

pub fn preview_piece_fill(kind: Option<TetrominoType>) -> Rgba {
    kind.map_or_else(|| rgba(0x00000000), piece_color)
}
