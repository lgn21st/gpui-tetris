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

pub fn render_preview(
    ui: &mut UiState,
    kind: Option<TetrominoType>,
    cell_size: f32,
) -> impl IntoElement + use<> {
    const PREVIEW_SIZE: usize = 4;
    let filled = ui.preview_mask(kind);

    let mut rows = Vec::with_capacity(PREVIEW_SIZE as usize);
    for y in 0..PREVIEW_SIZE {
        let mut row = div().flex();
        for x in 0..PREVIEW_SIZE {
            let cell_kind = if filled[y][x] { kind } else { None };
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
