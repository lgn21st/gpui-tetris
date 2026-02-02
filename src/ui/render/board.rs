use gpui::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::ui_state::UiState;
use gpui_tetris::game::pieces::TetrominoType;

pub fn render_cell(
    kind: Option<TetrominoType>,
    ghost: bool,
    flash: bool,
    cell_size: f32,
) -> impl IntoElement {
    let fill = theme::piece_fill(kind, ghost);
    let border = if flash {
        theme::flash_border()
    } else {
        theme::ghost_fill()
    };

    div()
        .w(px(cell_size))
        .h(px(cell_size))
        .bg(fill)
        .border(px(1.0))
        .border_color(border)
}

pub fn render_active_piece(
    kind: TetrominoType,
    blocks: &[(i32, i32); 4],
    offset_x: f32,
    offset_y: f32,
    cell_size: f32,
    opacity: f32,
) -> impl IntoElement {
    let fill = theme::piece_fill(Some(kind), false);
    let border = theme::ghost_fill();
    let mut layer = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .opacity(opacity);

    for (dx, dy) in blocks.iter() {
        let left = offset_x + (*dx as f32 * cell_size);
        let top = offset_y + (*dy as f32 * cell_size);
        layer = layer.child(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(cell_size))
                .h(px(cell_size))
                .bg(fill)
                .border(px(1.0))
                .border_color(border),
        );
    }

    layer
}

pub fn render_preview(
    ui: &mut UiState,
    kind: Option<TetrominoType>,
    cell_size: f32,
) -> impl IntoElement + use<> {
    const PREVIEW_SIZE: usize = 4;
    let filled = ui.preview_mask(kind);
    let mut rows = Vec::with_capacity(PREVIEW_SIZE);
    for row_filled in filled.iter().take(PREVIEW_SIZE) {
        let mut row = div().flex();
        for &cell_filled in row_filled.iter().take(PREVIEW_SIZE) {
            let cell_kind = if cell_filled { kind } else { None };
            row = row.child(render_preview_cell(cell_kind, cell_size));
        }
        rows.push(row);
    }

    div()
        .bg(theme::app_bg())
        .border(px(1.0))
        .border_color(theme::ghost_fill())
        .child(div().flex().flex_col().children(rows))
}

pub fn render_preview_compact(
    ui: &mut UiState,
    kind: Option<TetrominoType>,
    cell_size: f32,
) -> impl IntoElement + use<> {
    const PREVIEW_SIZE: usize = 4;
    let filled = ui.preview_mask(kind);
    let bounds = preview_bounds(filled).unwrap_or((0, PREVIEW_SIZE - 1, 0, PREVIEW_SIZE - 1));
    let (min_x, max_x, min_y, max_y) = bounds;

    let mut rows = Vec::with_capacity(max_y - min_y + 1);
    for row_filled in filled.iter().take(max_y + 1).skip(min_y) {
        let mut row = div().flex();
        for &cell_filled in row_filled.iter().take(max_x + 1).skip(min_x) {
            let cell_kind = if cell_filled { kind } else { None };
            row = row.child(render_preview_cell(cell_kind, cell_size));
        }
        rows.push(row);
    }

    div()
        .bg(theme::app_bg())
        .border(px(1.0))
        .border_color(theme::ghost_fill())
        .child(div().flex().flex_col().children(rows))
}

fn preview_bounds(mask: &[[bool; 4]; 4]) -> Option<(usize, usize, usize, usize)> {
    let mut min_x = 4;
    let mut min_y = 4;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for (y, row) in mask.iter().enumerate() {
        for (x, filled) in row.iter().enumerate() {
            if *filled {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if found {
        Some((min_x, max_x, min_y, max_y))
    } else {
        None
    }
}

fn render_preview_cell(kind: Option<TetrominoType>, cell_size: f32) -> impl IntoElement {
    let size = cell_size * 0.6;
    let color = theme::piece_fill(kind, false);

    div()
        .w(px(size))
        .h(px(size))
        .bg(color)
        .border(px(1.0))
        .border_color(theme::ghost_fill())
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
        .bg(gpui::rgb(0xffffff))
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
        .bg(theme::game_over_tint())
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
        .bg(theme::lock_warning())
        .opacity(intensity)
}
