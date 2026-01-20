use gpui::{IntoElement, div, prelude::*, px, rgb};

use gpui_tetris::game::input::GameAction;

use crate::ui::render::{
    render_cell, render_game_over_tint, render_line_clear_flash, render_lock_bar,
    render_lock_warning, render_overlay, render_preview,
};
use crate::ui::style::{BASE_PANEL_TEXT, BOARD_COLS, BOARD_ROWS};
use crate::ui::ui_state::UiState;

pub fn render_board(
    ui: &UiState,
    cell_size: f32,
    board_width: f32,
    board_height: f32,
    focused: bool,
    scale: f32,
) -> impl IntoElement {
    let show_active = !ui.state.is_line_clear_active();
    let mut active_cells = Vec::with_capacity(4);
    let landing_cells = if ui.state.landing_flash_active() {
        ui.state.last_lock_cells
    } else {
        [(0, 0); 4]
    };
    let ghost_cells = if show_active {
        let active_blocks = ui.state.active.blocks(ui.state.active.rotation);
        active_cells = Vec::with_capacity(4);
        for (dx, dy) in active_blocks.iter() {
            active_cells.push((ui.state.active.x + dx, ui.state.active.y + dy));
        }
        ui.state.ghost_blocks()
    } else {
        [(0, 0); 4]
    };

    let mut rows = Vec::with_capacity(BOARD_ROWS as usize);
    for y in 0..BOARD_ROWS as i32 {
        let mut row = div().flex();
        for x in 0..BOARD_COLS as i32 {
            let mut cell_kind = ui.state.board.cells[y as usize][x as usize].kind;
            let mut is_ghost = false;
            let mut is_flash = false;
            if ui.state.landing_flash_active()
                && landing_cells.iter().any(|(lx, ly)| *lx == x && *ly == y)
            {
                is_flash = true;
            }
            if show_active && active_cells.iter().any(|(ax, ay)| *ax == x && *ay == y) {
                cell_kind = Some(ui.state.active.kind);
            } else if show_active && ghost_cells.iter().any(|(gx, gy)| *gx == x && *gy == y) {
                cell_kind = Some(ui.state.active.kind);
                is_ghost = true;
            }

            row = row.child(render_cell(cell_kind, is_ghost, is_flash, cell_size));
        }
        rows.push(row);
    }

    div()
        .w(px(board_width))
        .h(px(board_height))
        .bg(rgb(0x1c1c1c))
        .border(px(1.0))
        .border_color(rgb(0x2e2e2e))
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
            scale,
        ))
}

pub fn render_panel(
    ui: &UiState,
    cell_size: f32,
    board_height: f32,
    panel_width: f32,
    padding: f32,
    gap: f32,
    scale: f32,
) -> impl IntoElement {
    div()
        .w(px(panel_width.max(cell_size * 4.0)))
        .h(px(board_height))
        .bg(rgb(0x1a1a1a))
        .border(px(1.0))
        .border_color(rgb(0x2e2e2e))
        .p(px(padding * 0.75))
        .flex()
        .flex_col()
        .gap(px(gap * 0.6))
        .text_size(px(BASE_PANEL_TEXT * scale))
        .text_color(rgb(0xe6e6e6))
        .child(
            div()
                .text_size(px(BASE_PANEL_TEXT * scale * 0.95))
                .child(format!(
                    "Last input: {}",
                    ui.last_action.as_ref().map(action_label).unwrap_or("None")
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(gap * 0.2))
                .child(format!("Score: {}", ui.state.score))
                .child(format!("Level: {}", ui.state.level))
                .child(format!("Lines: {}", ui.state.lines))
                .child(format!("Status: {}", ui.status_label()))
                .child(format!("Rules: {}", ui.ruleset_label()))
                .child(format!(
                    "Hold: {}",
                    if ui.state.can_hold { "Ready" } else { "Used" }
                ))
                .child(format!(
                    "Grounded: {}",
                    if ui.state.is_grounded() { "Yes" } else { "No" }
                ))
                .child(format!(
                    "Lock resets: {}/{}",
                    ui.state.lock_reset_remaining(),
                    ui.state.lock_reset_limit
                ))
                .child(format!("SFX: {}", ui.sfx_volume_label()))
                .child(if ui.state.is_classic_ruleset() {
                    div().hidden()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(format!(
                            "Combo: {}",
                            if ui.state.combo >= 0 {
                                ui.state.combo.to_string()
                            } else {
                                "-".to_string()
                            }
                        ))
                        .child(format!(
                            "B2B: {}",
                            if ui.state.back_to_back { "Yes" } else { "No" }
                        ))
                        .child(if ui.state.back_to_back {
                            div()
                                .text_sm()
                                .text_color(rgb(0xfacc15))
                                .child("B2B bonus active")
                        } else {
                            div().hidden()
                        })
                })
                .child(render_lock_bar(
                    ui.state.lock_timer_ms,
                    ui.state.lock_delay_ms,
                    ui.state.is_grounded(),
                    scale,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(gap * 0.2))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_TEXT * scale * 0.95))
                        .child("Hold"),
                )
                .child(render_preview(ui.state.hold.as_ref(), cell_size)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(gap * 0.2))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_TEXT * scale * 0.95))
                        .child("Next"),
                )
                .child(render_preview(ui.state.next_queue.first(), cell_size)),
        )
}

fn action_label(action: &GameAction) -> &'static str {
    match action {
        GameAction::MoveLeft => "Left",
        GameAction::MoveRight => "Right",
        GameAction::SoftDrop => "Soft Drop",
        GameAction::HardDrop => "Hard Drop",
        GameAction::RotateCw => "Rotate CW",
        GameAction::RotateCcw => "Rotate CCW",
        GameAction::Hold => "Hold",
        GameAction::Pause => "Pause",
        GameAction::Restart => "Restart",
    }
}
