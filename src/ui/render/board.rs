use gpui_kit::{IntoElement, div, prelude::*, px, rgba};

use crate::ui::render::theme;
use crate::ui::style::{PREVIEW_CORNER_RADIUS, PREVIEW_GAP, PREVIEW_PADDING};
use crate::ui::ui_state::UiState;
use gpui_tetris::game::pieces::TetrominoType;

pub fn render_cell(
    kind: Option<TetrominoType>,
    ghost: bool,
    flash: bool,
    cell_size: f32,
    stroke_width: f32,
) -> impl IntoElement {
    let fill = kind.map_or_else(|| rgba(0x00000000), |_| theme::piece_fill(kind, ghost));
    let border = if flash {
        theme::flash_border()
    } else {
        theme::gridline()
    };

    div()
        .w(px(cell_size))
        .h(px(cell_size))
        .bg(fill)
        .when(ghost || flash, |cell| {
            cell.border(px(stroke_width)).border_color(border)
        })
}

pub fn render_board_grid(
    columns: usize,
    rows: usize,
    cell_size: f32,
    stroke_width: f32,
) -> impl IntoElement {
    let mut grid = div().absolute().top_0().left_0().right_0().bottom_0();
    for column in 1..columns {
        grid = grid.child(
            div()
                .absolute()
                .left(px(column as f32 * cell_size - stroke_width / 2.0))
                .top_0()
                .bottom_0()
                .w(px(stroke_width))
                .bg(theme::gridline()),
        );
    }
    for row in 1..rows {
        grid = grid.child(
            div()
                .absolute()
                .top(px(row as f32 * cell_size - stroke_width / 2.0))
                .left_0()
                .right_0()
                .h(px(stroke_width))
                .bg(theme::gridline()),
        );
    }
    grid
}

pub fn render_active_piece(
    kind: TetrominoType,
    blocks: &[(i32, i32); 4],
    offset_x: f32,
    offset_y: f32,
    cell_size: f32,
    stroke_width: f32,
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
                .border(px(stroke_width))
                .border_color(border),
        );
    }

    layer
}

pub fn render_preview(
    ui: &mut UiState,
    kind: Option<TetrominoType>,
    cell_size: f32,
    scale: f32,
) -> impl IntoElement + use<> {
    const PREVIEW_SIZE: usize = 4;
    let filled = ui.preview_mask(kind);
    let mut rows = Vec::with_capacity(PREVIEW_SIZE);
    for row_filled in filled.iter().take(PREVIEW_SIZE) {
        let mut row = div().flex().gap(px(PREVIEW_GAP * scale));
        for &cell_filled in row_filled.iter().take(PREVIEW_SIZE) {
            let cell_kind = if cell_filled { kind } else { None };
            row = row.child(render_preview_cell(cell_kind, cell_size, scale));
        }
        rows.push(row);
    }

    div()
        .p(px(PREVIEW_PADDING * scale))
        .rounded(px(PREVIEW_CORNER_RADIUS * scale))
        .bg(theme::preview_bg())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(PREVIEW_GAP * scale))
                .children(rows),
        )
}

fn render_preview_cell(
    kind: Option<TetrominoType>,
    cell_size: f32,
    stroke_width: f32,
) -> impl IntoElement {
    let color = theme::preview_piece_fill(kind);

    div()
        .w(px(cell_size))
        .h(px(cell_size))
        .bg(color)
        .border(px(stroke_width))
        .border_color(theme::preview_gridline())
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
        .bg(gpui_kit::rgb(0xffffff))
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
