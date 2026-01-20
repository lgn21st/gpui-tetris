use gpui::{IntoElement, div, prelude::*, px, rgb};

use crate::ui::style::{BASE_HINT_TEXT, BASE_TITLE_TEXT};

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
            .bg(rgb(0x000000))
            .opacity(0.86)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(rgb(0xf5f5f5))
            .text_size(px(title_size))
            .child("Settings")
            .child(div().text_size(px(hint_size)).child(format!(
                "SFX Volume: {}{}",
                sfx_label,
                if muted { " (M)" } else { "" }
            )))
            .child(
                div()
                    .text_size(px(hint_size))
                    .child("M: mute · +/-: volume · 0: reset"),
            )
            .child(div().text_size(px(hint_size)).child("S or Esc: back"));
    }

    if !started {
        return div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .bg(rgb(0x000000))
            .opacity(0.86)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(rgb(0xf5f5f5))
            .text_size(px(title_size))
            .child("gpui‑tetris")
            .child(
                div()
                    .text_size(px(hint_size))
                    .child("Press Enter or Space to Start"),
            )
            .child(div().text_size(px(hint_size)).child("S: Settings"));
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
            .bg(rgb(0x000000))
            .opacity(0.78)
            .flex()
            .flex_col()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(rgb(0xf5f5f5))
            .text_size(px(title_size))
            .child("Click to Focus");
    }

    let label = if game_over { "Game Over" } else { "Paused" };
    let hint = if game_over {
        "Press R to restart"
    } else {
        "Press P to resume"
    };

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0x000000))
        .opacity(0.82)
        .flex()
        .flex_col()
        .gap_2()
        .justify_center()
        .items_center()
        .text_color(rgb(0xf5f5f5))
        .text_size(px(title_size))
        .child(label)
        .child(div().text_size(px(hint_size)).child(hint))
}
