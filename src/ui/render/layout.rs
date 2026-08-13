use gpui::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::render::{
    OverlayState, render_active_piece, render_cell, render_game_over_tint, render_line_clear_flash,
    render_lock_bar, render_lock_warning, render_overlay, render_preview, render_preview_compact,
};
use crate::ui::style::{BASE_GAP, BASE_PADDING, BASE_PANEL_TEXT, CELL_SIZE, WINDOW_WIDTH};
use crate::ui::ui_state::UiState;
use gpui_tetris::game::board::{BOARD_HEIGHT, BOARD_WIDTH};

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
        let cell_size = CELL_SIZE * scale;
        let padding = BASE_PADDING * scale;
        let gap = BASE_GAP * scale;
        let board_width = cell_size * BOARD_WIDTH as f32;
        let board_height = cell_size * BOARD_HEIGHT as f32;
        let panel_width = (WINDOW_WIDTH * scale) - board_width - (padding * 2.0) - gap;
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
    now: std::time::Instant,
) -> impl IntoElement + use<> {
    ui.sync_board_cache();
    let show_active = !ui.state.is_line_clear_active();
    let show_ghost = show_active && !ui.state.is_grounded() && ui.state.active_moved_since_spawn;
    let cols = BOARD_WIDTH as i32;
    let rows = BOARD_HEIGHT as i32;
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

    if show_ghost {
        for (x, y) in ui.state.ghost_blocks().iter() {
            set_mask(&mut ui.ghost_mask, *x, *y);
        }
    }

    let mut rows = Vec::with_capacity(BOARD_HEIGHT);
    for y in 0..BOARD_HEIGHT {
        let mut row = div().flex();
        let row_base = y * BOARD_WIDTH;
        for x in 0..BOARD_WIDTH {
            let idx = row_base + x;
            let mut cell_kind = ui.board_cache[idx];
            let mut is_ghost = false;
            let is_flash = ui.flash_mask[idx];

            if show_ghost && ui.ghost_mask[idx] {
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
        .child(render_active_overlay(ui, layout, show_active, now))
        .child(render_line_clear_flash(ui.state.line_clear_timer_ms > 0))
        .child(render_lock_warning(if ui.state.is_grounded() {
            ui.state.lock_warning_intensity()
        } else {
            0.0
        }))
        .child(render_game_over_tint(ui.state.game_over))
        .child(render_overlay(&OverlayState {
            started: ui.started,
            show_settings: ui.show_settings,
            paused: ui.state.paused,
            game_over: ui.state.game_over,
            focused,
            scale: layout.scale,
        }))
}

fn render_active_overlay(
    ui: &UiState,
    layout: &RenderLayout,
    show_active: bool,
    now: std::time::Instant,
) -> impl IntoElement + use<> {
    if !show_active {
        return div().hidden();
    }

    let active = ui.active_snapshot();
    let cell_size = layout.cell_size;
    let mut layer = div().absolute().top_0().left_0().right_0().bottom_0();

    if let Some(anim) = ui.active_animation_state(now) {
        let progress = anim.progress;
        let dx = (anim.from_x - anim.to_x) as f32;
        let dy = (anim.from_y - anim.to_y) as f32;
        let offset_x = ((anim.to_x as f32) + dx * (1.0 - progress)) * cell_size;
        let offset_y = ((anim.to_y as f32) + dy * (1.0 - progress)) * cell_size;

        let to_piece = gpui_tetris::game::pieces::Tetromino::new(anim.kind, 0, 0);
        let to_blocks = to_piece.blocks(anim.to_rotation);
        layer = layer.child(render_active_piece(
            anim.kind, &to_blocks, offset_x, offset_y, cell_size, 1.0,
        ));

        if anim.rotation_changed && anim.from_rotation != anim.to_rotation {
            let from_piece = gpui_tetris::game::pieces::Tetromino::new(anim.kind, 0, 0);
            let from_blocks = from_piece.blocks(anim.from_rotation);
            let from_offset_x = anim.from_x as f32 * cell_size;
            let from_offset_y = anim.from_y as f32 * cell_size;
            layer = layer.child(render_active_piece(
                anim.kind,
                &from_blocks,
                from_offset_x,
                from_offset_y,
                cell_size,
                (1.0 - progress).clamp(0.0, 1.0),
            ));
        }
    } else {
        let piece = gpui_tetris::game::pieces::Tetromino::new(active.kind, 0, 0);
        let blocks = piece.blocks(active.rotation);
        let offset_x = active.x as f32 * cell_size;
        let offset_y = active.y as f32 * cell_size;
        layer = layer.child(render_active_piece(
            active.kind,
            &blocks,
            offset_x,
            offset_y,
            cell_size,
            1.0,
        ));
    }

    layer
}

pub fn render_panel(ui: &mut UiState, layout: &RenderLayout) -> impl IntoElement + use<> {
    let next_1 = ui.state.next_queue.first().copied();
    let next_2 = ui.state.next_queue.get(1).copied();
    let next_3 = ui.state.next_queue.get(2).copied();
    let next_gap = layout.gap * 0.35;
    let panel_width = layout.panel_width.max(layout.cell_size * 4.0);
    let panel_inner_width = (panel_width - layout.padding * 1.5).max(1.0);
    // render_preview uses a 4x4 grid where each cell is 0.6 * input size.
    let preview_width_factor = 4.0 * 0.6;
    let fit_size = (panel_inner_width - next_gap * 2.0) / (3.0 * preview_width_factor);
    let next_preview_cell = fit_size.clamp(layout.cell_size * 0.45, layout.cell_size * 1.05);

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
                .child(render_preview_compact(ui, ui.state.hold, layout.cell_size)),
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
                .child(
                    div()
                        .flex()
                        .gap(px(next_gap))
                        .child(render_preview(ui, next_1, next_preview_cell))
                        .child(render_preview(ui, next_2, next_preview_cell))
                        .child(render_preview(ui, next_3, next_preview_cell)),
                ),
        )
}
