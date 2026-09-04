use gpui_kit::{IntoElement, div, prelude::*, px};

use crate::ui::render::theme;

pub fn render_frame(stroke_width: f32) -> impl IntoElement {
    let edge_color = theme::border();
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
                .h(px(stroke_width))
                .bg(edge_color),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(stroke_width))
                .bg(edge_color),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .w(px(stroke_width))
                .bg(edge_color),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(stroke_width))
                .bg(edge_color),
        )
}
