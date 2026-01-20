use gpui::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::render::{
    render_cell, render_game_over_tint, render_line_clear_flash, render_lock_bar,
    render_lock_warning, render_overlay, render_preview,
};
use crate::ui::style::{
    BASE_CELL_SIZE, BASE_GAP, BASE_PADDING, BASE_PANEL_TEXT, BASE_WINDOW_WIDTH, BOARD_COLS,
    BOARD_COLS_USIZE, BOARD_ROWS, BOARD_ROWS_USIZE,
};
use crate::ui::ui_state::UiState;

pub struct RenderLayout {
    pub scale: f32,
    pub cell_size: f32,
    pub padding: f32,
    pub gap: f32,
    pub board_width: f32,
    pub board_height: f32,
    pub panel_width: f32,
}

impl RenderLayout {
    pub fn new(scale: f32) -> Self {
        let cell_size = BASE_CELL_SIZE * scale;
        let padding = BASE_PADDING * scale;
        let gap = BASE_GAP * scale;
        let board_width = cell_size * BOARD_COLS;
        let board_height = cell_size * BOARD_ROWS;
        let panel_width = (BASE_WINDOW_WIDTH * scale) - board_width - (padding * 2.0) - gap;
        Self {
            scale,
            cell_size,
            padding,
            gap,
            board_width,
            board_height,
            panel_width,
        }
    }
}

pub fn render_board(
    ui: &mut UiState,
    layout: &RenderLayout,
    focused: bool,
) -> impl IntoElement + use<> {
    ui.sync_board_cache();
    let show_active = !ui.state.is_line_clear_active();
    let cols = BOARD_COLS_USIZE as i32;
    let rows = BOARD_ROWS_USIZE as i32;
    ui.clear_render_masks();

    let set_mask = |mask: &mut [bool], x: i32, y: i32| {
        if x >= 0 && x < cols && y >= 0 && y < rows {
            let idx = (y as usize * cols as usize) + x as usize;
            mask[idx] = true;
        }
    };

    if ui.state.landing_flash_active() {
        for (x, y) in ui.state.last_lock_cells.iter() {
            set_mask(&mut ui.flash_mask, *x, *y);
        }
    }

    if show_active {
        for (dx, dy) in ui.state.active.blocks(ui.state.active.rotation).iter() {
            set_mask(
                &mut ui.active_mask,
                ui.state.active.x + dx,
                ui.state.active.y + dy,
            );
        }
        for (x, y) in ui.state.ghost_blocks().iter() {
            set_mask(&mut ui.ghost_mask, *x, *y);
        }
    }

    let mut rows = Vec::with_capacity(BOARD_ROWS_USIZE);
    for y in 0..BOARD_ROWS_USIZE as i32 {
        let mut row = div().flex();
        for x in 0..BOARD_COLS_USIZE as i32 {
            let idx = (y as usize * cols as usize) + x as usize;
            let mut cell_kind = ui.board_cache[idx];
            let mut is_ghost = false;
            let is_flash = ui.flash_mask[idx];

            if show_active && ui.active_mask[idx] {
                cell_kind = Some(ui.state.active.kind);
            } else if show_active && ui.ghost_mask[idx] {
                cell_kind = Some(ui.state.active.kind);
                is_ghost = true;
            }

            row = row.child(render_cell(cell_kind, is_ghost, is_flash, layout.cell_size));
        }
        rows.push(row);
    }

    div()
        .w(px(layout.board_width))
        .h(px(layout.board_height))
        .bg(theme::board_bg())
        .border(px(1.0))
        .border_color(theme::border())
        .relative()
        .child(div().flex().flex_col().children(rows))
        .child(render_line_clear_flash(ui.state.line_clear_timer_ms > 0))
        .child(render_lock_warning(ui.state.lock_warning_intensity()))
        .child(render_game_over_tint(ui.state.game_over))
        .child(render_overlay(
            ui.started,
            ui.show_settings,
            ui.state.paused,
            ui.state.game_over,
            focused,
            ui.sfx_volume_label(),
            ui.sfx_muted,
            layout.scale,
        ))
}

pub fn render_panel(ui: &mut UiState, layout: &RenderLayout) -> impl IntoElement + use<> {
    div()
        .w(px(layout.panel_width.max(layout.cell_size * 4.0)))
        .h(px(layout.board_height))
        .bg(theme::panel_bg())
        .border(px(1.0))
        .border_color(theme::border())
        .p(px(layout.padding * 0.75))
        .flex()
        .flex_col()
        .gap(px(layout.gap * 0.6))
        .text_size(px(BASE_PANEL_TEXT * layout.scale))
        .text_color(theme::panel_text())
        .child(
            div()
                .text_size(px(BASE_PANEL_TEXT * layout.scale * 0.95))
                .child(ui.panel_labels.last_input.clone()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(layout.gap * 0.2))
                .child(ui.panel_labels.score.clone())
                .child(ui.panel_labels.level.clone())
                .child(ui.panel_labels.lines.clone())
                .child(ui.panel_labels.status.clone())
                .child(ui.panel_labels.ruleset.clone())
                .child(ui.panel_labels.hold.clone())
                .child(ui.panel_labels.grounded.clone())
                .child(ui.panel_labels.lock_resets.clone())
                .child(ui.panel_labels.sfx.clone())
                .child(if ui.state.is_classic_ruleset() {
                    div().hidden()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui.panel_labels.combo.clone())
                        .child(ui.panel_labels.b2b.clone())
                        .child(if ui.state.back_to_back {
                            div()
                                .text_sm()
                                .text_color(theme::b2b_text())
                                .child("B2B bonus active")
                        } else {
                            div().hidden()
                        })
                })
                .child(render_lock_bar(
                    ui.state.lock_timer_ms,
                    ui.state.lock_delay_ms,
                    ui.state.is_grounded(),
                    layout.scale,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(layout.gap * 0.2))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_TEXT * layout.scale * 0.95))
                        .child("Hold"),
                )
                .child(render_preview(ui, ui.state.hold, layout.cell_size)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(layout.gap * 0.2))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_TEXT * layout.scale * 0.95))
                        .child("Next"),
                )
                .child(render_preview(
                    ui,
                    ui.state.next_queue.first().copied(),
                    layout.cell_size,
                )),
        )
}
