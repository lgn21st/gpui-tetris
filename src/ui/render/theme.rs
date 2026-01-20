use gpui::{Rgba, rgb};

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

pub fn ghost_fill() -> Rgba {
    rgb(0x2a2a2a)
}

pub fn flash_border() -> Rgba {
    rgb(0xfef3c7)
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

pub fn lock_warning() -> Rgba {
    rgb(0x7a1c1c)
}

pub fn lock_bar_bg() -> Rgba {
    rgb(0x1f2937)
}

pub fn lock_bar_border() -> Rgba {
    rgb(0x374151)
}

pub fn lock_bar_safe() -> Rgba {
    rgb(0x34d399)
}

pub fn lock_bar_warn() -> Rgba {
    rgb(0xfbbf24)
}

pub fn lock_bar_danger() -> Rgba {
    rgb(0xf87171)
}

pub fn panel_text() -> Rgba {
    rgb(0xe6e6e6)
}

pub fn b2b_text() -> Rgba {
    rgb(0xfacc15)
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
        None => app_bg(),
    }
}
