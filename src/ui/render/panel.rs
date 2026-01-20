use gpui::{IntoElement, div, prelude::*, px, rgb};

use crate::ui::style::BASE_PANEL_TEXT;

pub fn render_lock_bar(
    lock_timer_ms: u64,
    lock_delay_ms: u64,
    grounded: bool,
    scale: f32,
) -> impl IntoElement {
    const BAR_WIDTH: f32 = 140.0;
    const BAR_HEIGHT: f32 = 6.0;

    if !grounded || lock_delay_ms == 0 {
        return div().hidden();
    }

    let bar_width = BAR_WIDTH * scale;
    let bar_height = BAR_HEIGHT * scale;
    let ratio = (lock_timer_ms as f32 / lock_delay_ms as f32).clamp(0.0, 1.0);
    let fill_width = bar_width * ratio;
    let fill_color = if ratio > 0.8 {
        rgb(0xf87171)
    } else if ratio > 0.5 {
        rgb(0xfbbf24)
    } else {
        rgb(0x34d399)
    };

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(BASE_PANEL_TEXT * scale * 0.95))
                .child("Lock delay"),
        )
        .child(
            div()
                .w(px(bar_width))
                .h(px(bar_height))
                .bg(rgb(0x1f2937))
                .border(px(1.0))
                .border_color(rgb(0x374151))
                .child(div().w(px(fill_width)).h(px(bar_height)).bg(fill_color)),
        )
}
