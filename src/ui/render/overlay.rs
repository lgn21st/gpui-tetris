use gpui_kit::{FontWeight, IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
use crate::ui::style::{BASE_HINT_TEXT, BASE_ONBOARDING_TEXT, BASE_TITLE_TEXT};
use crate::ui::ui_state::{GAME_OVER_HINT, ONBOARDING_HINTS, PAUSED_HINT, TITLE_HINT};

pub fn render_overlay(state: &OverlayState) -> impl IntoElement + use<> {
    let title_size = (BASE_TITLE_TEXT * state.scale).max(16.0);
    let hint_size = (BASE_HINT_TEXT * state.scale).max(10.0);
    let onboarding_size = (BASE_ONBOARDING_TEXT * state.scale).max(9.0);
    if state.show_settings {
        return div().hidden();
    }

    if !state.paused && !state.game_over && state.started {
        return div().hidden();
    }

    let (label, hint) = if !state.started {
        ("GPUI Tetris", TITLE_HINT)
    } else if state.game_over {
        ("Game Over", GAME_OVER_HINT)
    } else {
        ("Paused", PAUSED_HINT)
    };

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(theme::overlay_bg())
                .opacity(0.55),
        )
        .child(
            div()
                .size_full()
                .relative()
                .flex()
                .flex_col()
                .gap_2()
                .justify_center()
                .items_center()
                .text_color(theme::overlay_text())
                .child(
                    div()
                        .text_size(px(title_size))
                        .font_weight(FontWeight::BOLD)
                        .child(label),
                )
                .child(div().text_size(px(hint_size)).child(hint))
                .when(!state.started, |content| {
                    content.child(
                        div()
                            .mt_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .items_center()
                            .text_size(px(onboarding_size))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme::secondary_text())
                            .children(ONBOARDING_HINTS),
                    )
                }),
        )
}

pub struct OverlayState {
    pub started: bool,
    pub show_settings: bool,
    pub paused: bool,
    pub game_over: bool,
    pub scale: f32,
}
