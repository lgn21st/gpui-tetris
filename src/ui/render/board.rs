use gpui::{IntoElement, div, prelude::*, px, rgb};

use crate::ui::style;
use gpui_tetris::game::pieces::{Tetromino, TetrominoType};

pub fn render_cell(
    kind: Option<TetrominoType>,
    ghost: bool,
    flash: bool,
    cell_size: f32,
) -> impl IntoElement {
    let fill = match kind {
        Some(piece) => {
            if ghost {
                rgb(0x2a2a2a)
            } else {
                style::piece_color(piece)
            }
        }
        None => rgb(0x101010),
    };
    let border = if flash { rgb(0xfef3c7) } else { rgb(0x2a2a2a) };

    div()
        .w(px(cell_size))
        .h(px(cell_size))
        .bg(fill)
        .border(px(1.0))
        .border_color(border)
}

pub fn render_preview(kind: Option<&TetrominoType>, cell_size: f32) -> impl IntoElement {
    const PREVIEW_SIZE: i32 = 4;
    let mut filled = [[false; PREVIEW_SIZE as usize]; PREVIEW_SIZE as usize];

    if let Some(kind) = kind {
        let piece = Tetromino::new(*kind, 0, 0);
        for (x, y) in piece.blocks(piece.rotation).iter() {
            let ux = *x as usize;
            let uy = *y as usize;
            if ux < PREVIEW_SIZE as usize && uy < PREVIEW_SIZE as usize {
                filled[uy][ux] = true;
            }
        }
    }

    let mut rows = Vec::new();
    for y in 0..PREVIEW_SIZE {
        let mut row = div().flex();
        for x in 0..PREVIEW_SIZE {
            let cell_kind = if filled[y as usize][x as usize] {
                kind.copied()
            } else {
                None
            };
            row = row.child(render_preview_cell(cell_kind, cell_size));
        }
        rows.push(row);
    }

    div()
        .bg(rgb(0x101010))
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
        .child(div().flex().flex_col().children(rows))
}

fn render_preview_cell(kind: Option<TetrominoType>, cell_size: f32) -> impl IntoElement {
    let size = cell_size * 0.6;
    let color = match kind {
        Some(piece) => style::piece_color(piece),
        None => rgb(0x101010),
    };

    div()
        .w(px(size))
        .h(px(size))
        .bg(color)
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
}

pub fn render_line_clear_flash(active: bool) -> impl IntoElement {
    if !active {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0xffffff))
        .opacity(0.12)
}

pub fn render_game_over_tint(active: bool) -> impl IntoElement {
    if !active {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0x3a0f0f))
        .opacity(0.28)
}

pub fn render_lock_warning(intensity: f32) -> impl IntoElement {
    if intensity <= 0.0 {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0x7a1c1c))
        .opacity(intensity)
}
