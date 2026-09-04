use gpui_kit::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;
pub fn render_lock_bar(
    lock_timer_ms: u64,
    lock_delay_ms: u64,
    grounded: bool,
    bar_width: f32,
    scale: f32,
) -> impl IntoElement {
    const BAR_HEIGHT: f32 = 6.0;

    let active = grounded && lock_delay_ms > 0;
    let bar_height = BAR_HEIGHT * scale;
    let ratio = if active {
        (lock_timer_ms as f32 / lock_delay_ms as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill_width = bar_width * ratio;
    let fill_color = if ratio >= 0.85 {
        theme::lock_bar_danger()
    } else {
        theme::lock_bar_safe()
    };

    div()
        .w(px(bar_width))
        .h(px(bar_height))
        .rounded(px(bar_height / 2.0))
        .bg(theme::lock_bar_bg())
        .child(
            div()
                .w(px(fill_width))
                .h(px(bar_height))
                .rounded(px(bar_height / 2.0))
                .bg(fill_color),
        )
}
