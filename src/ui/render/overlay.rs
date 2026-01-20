use gpui::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::style::{BASE_HINT_TEXT, BASE_TITLE_TEXT};
use crate::ui::ui_state::{
    FOCUS_HINT, GAME_OVER_HINT, PAUSED_HINT, SETTINGS_BACK, SETTINGS_SHORTCUTS, TITLE_HINT,
    TITLE_SETTINGS,
};

pub fn render_overlay(
    started: bool,
    show_settings: bool,
    paused: bool,
    game_over: bool,
    focused: bool,
    sfx_label: String,
    muted: bool,
    scale: f32,
) -> impl IntoElement {
    let title_size = (BASE_TITLE_TEXT * scale).max(16.0);
    let hint_size = (BASE_HINT_TEXT * scale).max(10.0);
    if show_settings {
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(theme::overlay_bg())
            .opacity(0.86)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(theme::overlay_text())
            .text_size(px(title_size))
            .child("Settings")
            .child(div().text_size(px(hint_size)).child(format!(
                "SFX Volume: {}{}",
                sfx_label,
                if muted { " (M)" } else { "" }
            )))
            .child(div().text_size(px(hint_size)).child(SETTINGS_SHORTCUTS))
            .child(div().text_size(px(hint_size)).child(SETTINGS_BACK));
    }

    if !started {
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(theme::overlay_bg())
            .opacity(0.86)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(theme::overlay_text())
            .text_size(px(title_size))
            .child("gpui‑tetris")
            .child(div().text_size(px(hint_size)).child(TITLE_HINT))
            .child(div().text_size(px(hint_size)).child(TITLE_SETTINGS));
    }

    if !paused && !game_over {
        if focused {
            return div().hidden();
        }
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(theme::overlay_bg())
            .opacity(0.78)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(theme::overlay_text())
            .text_size(px(title_size))
            .child(FOCUS_HINT);
    }

    let label = if game_over { "Game Over" } else { "Paused" };
    let hint = if game_over {
        GAME_OVER_HINT
    } else {
        PAUSED_HINT
    };

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(theme::overlay_bg())
        .opacity(0.82)
        .flex()
        .flex_col()
        .gap_2()
        .justify_center()
        .items_center()
        .text_color(theme::overlay_text())
        .text_size(px(title_size))
        .child(label)
        .child(div().text_size(px(hint_size)).child(hint))
}
