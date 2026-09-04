use gpui_kit::{FontWeight, IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::render::{
    render_active_piece, render_board_grid, render_cell, render_frame, render_game_over_tint,
    render_line_clear_flash, render_lock_bar, render_preview,
};
use crate::ui::style::{
    BASE_GAP, BASE_PANEL_SECTION_TEXT, BASE_PANEL_TEXT, CELL_SIZE, CONTENT_WIDTH,
    NEXT_PREVIEW_CELL, PANEL_DIVIDER_HEIGHT, PANEL_DIVIDER_PADDING, PANEL_ITEM_SPACING,
    PANEL_PADDING, PANEL_SECTION_SPACING, PANEL_WIDTH, PREVIEW_CELL,
};
use crate::ui::ui_state::UiState;
use gpui_tetris::game::board::{BOARD_HEIGHT, BOARD_WIDTH};

pub struct RenderLayout {
    pub scale: f32,
    pub cell_size: f32,
    pub gap: f32,
    pub board_width: f32,
    pub board_height: f32,
    pub panel_width: f32,
    pub panel_content_width: f32,
    pub content_width: f32,
    pub stroke_width: f32,
}

impl RenderLayout {
    pub fn new(scale: f32) -> Self {
        let cell_size = CELL_SIZE * scale;
        let gap = BASE_GAP * scale;
        debug_assert_eq!(
            CELL_SIZE * BOARD_WIDTH as f32,
            crate::ui::style::BOARD_WIDTH
        );
        debug_assert_eq!(
            CELL_SIZE * BOARD_HEIGHT as f32,
            crate::ui::style::BOARD_HEIGHT
        );
        let board_width = crate::ui::style::BOARD_WIDTH * scale;
        let board_height = crate::ui::style::BOARD_HEIGHT * scale;
        let panel_width = PANEL_WIDTH * scale;
        let panel_content_width = (PANEL_WIDTH - 2.0 * PANEL_PADDING) * scale;
        Self {
            scale,
            cell_size,
            gap,
            board_width,
            board_height,
            panel_width,
            panel_content_width,
            content_width: CONTENT_WIDTH * scale,
            stroke_width: scale,
        }
    }
}

pub fn render_board(
    ui: &mut UiState,
    layout: &RenderLayout,
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

            row = row.child(render_cell(
                cell_kind,
                is_ghost,
                is_flash,
                layout.cell_size,
                layout.stroke_width,
            ));
        }
        rows.push(row);
    }

    div()
        .w(px(layout.board_width))
        .h(px(layout.board_height))
        .bg(theme::board_bg())
        .relative()
        .overflow_hidden()
        .child(render_board_grid(
            BOARD_WIDTH,
            BOARD_HEIGHT,
            layout.cell_size,
            layout.stroke_width,
        ))
        .child(div().flex().flex_col().children(rows))
        .child(render_active_overlay(ui, layout, show_active, now))
        .child(render_line_clear_flash(ui.state.line_clear_timer_ms > 0))
        .child(render_game_over_tint(ui.state.game_over))
        .child(render_frame(layout.stroke_width))
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
            anim.kind,
            &to_blocks,
            offset_x,
            offset_y,
            cell_size,
            layout.stroke_width,
            1.0,
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
                layout.stroke_width,
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
            layout.stroke_width,
            1.0,
        ));
    }

    layer
}

pub fn render_panel(ui: &mut UiState, layout: &RenderLayout) -> impl IntoElement + use<> {
    let next_1 = ui.state.next_queue.first().copied();
    let next_2 = ui.state.next_queue.get(1).copied();
    let next_3 = ui.state.next_queue.get(2).copied();
    let next_gap = PANEL_ITEM_SPACING * layout.scale;
    let section_gap = PANEL_SECTION_SPACING * layout.scale;
    let item_gap = PANEL_ITEM_SPACING * layout.scale;
    let divider = || {
        div()
            .w_full()
            .h(px(PANEL_DIVIDER_HEIGHT * layout.scale))
            .my(px(PANEL_DIVIDER_PADDING * layout.scale))
            .bg(theme::divider())
    };

    div()
        .w(px(layout.panel_width))
        .h(px(layout.board_height))
        .bg(theme::panel_bg())
        .p(px(PANEL_PADDING * layout.scale))
        .relative()
        .flex()
        .flex_col()
        .gap(px(section_gap))
        .text_size(px(BASE_PANEL_TEXT * layout.scale))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::panel_text())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(item_gap))
                .child(ui.panel_labels.score.clone())
                .child(ui.panel_labels.level.clone())
                .child(ui.panel_labels.lines.clone())
                .child(ui.panel_labels.status.clone())
                .child(ui.panel_labels.ruleset.clone())
                .child(ui.panel_labels.hold.clone()),
        )
        .child(divider())
        .child(render_lock_bar(
            ui.state.lock_timer_ms,
            ui.state.lock_delay_ms,
            ui.state.is_grounded(),
            layout.panel_content_width,
            layout.scale,
        ))
        .child(divider())
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(section_gap))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_SECTION_TEXT * layout.scale))
                        .child("Hold"),
                )
                .child(render_preview(
                    ui,
                    ui.state.hold,
                    PREVIEW_CELL * layout.scale,
                    layout.scale,
                )),
        )
        .child(divider())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(item_gap))
                .child(
                    div()
                        .text_size(px(BASE_PANEL_SECTION_TEXT * layout.scale))
                        .child("Next"),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(next_gap))
                        .child(render_preview(
                            ui,
                            next_1,
                            NEXT_PREVIEW_CELL * layout.scale,
                            layout.scale,
                        ))
                        .child(render_preview(
                            ui,
                            next_2,
                            NEXT_PREVIEW_CELL * layout.scale,
                            layout.scale,
                        ))
                        .child(render_preview(
                            ui,
                            next_3,
                            NEXT_PREVIEW_CELL * layout.scale,
                            layout.scale,
                        )),
                ),
        )
        .child(render_frame(layout.stroke_width))
}

#[cfg(test)]
mod tests {
    use super::RenderLayout;
    use crate::ui::style::{
        BASE_GAP, BASE_PADDING, BOARD_HEIGHT, BOARD_WIDTH, PANEL_ITEM_SPACING,
        PANEL_SECTION_SPACING, PANEL_WIDTH, PREVIEW_CELL, WINDOW_HEIGHT, WINDOW_WIDTH,
    };

    #[test]
    fn base_layout_matches_design_geometry() {
        let layout = RenderLayout::new(1.0);

        assert_eq!(WINDOW_WIDTH, 480.0);
        assert_eq!(WINDOW_HEIGHT, 720.0);
        assert_eq!(BASE_PADDING, 16.0);
        assert_eq!(BASE_GAP, 16.0);
        assert_eq!(BOARD_WIDTH, 240.0);
        assert_eq!(BOARD_HEIGHT, 480.0);
        assert_eq!(PANEL_WIDTH, 192.0);
        assert_eq!(layout.board_width, BOARD_WIDTH);
        assert_eq!(layout.board_height, BOARD_HEIGHT);
        assert_eq!(layout.panel_width, PANEL_WIDTH);
        assert_eq!(layout.panel_content_width, 168.0);
        assert_eq!(layout.content_width, BOARD_WIDTH + BASE_GAP + PANEL_WIDTH);
        assert_eq!(layout.stroke_width, 1.0);
    }

    #[test]
    fn panel_spacing_matches_design_geometry() {
        assert_eq!(PANEL_SECTION_SPACING, 9.6);
        assert_eq!(PANEL_ITEM_SPACING, 3.2);
        assert_eq!(PREVIEW_CELL, 12.0);
    }
}
