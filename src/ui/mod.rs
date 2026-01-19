use gpui::{
    div, px, rgb, size, App, Application, Bounds, Context, IntoElement, Render, Window,
    WindowBounds, WindowOptions, prelude::*,
};

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const CELL_SIZE: f32 = 24.0;

const BOARD_COLS: f32 = 10.0;
const BOARD_ROWS: f32 = 20.0;
const PADDING: f32 = 16.0;
const GAP: f32 = 16.0;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            is_resizable: false,
            ..Default::default()
        };

        cx.open_window(options, |_, cx| cx.new(|_| TetrisView::new()))
            .unwrap();
        cx.activate(true);
    })
}

struct TetrisView {
    board_width: f32,
    board_height: f32,
    panel_width: f32,
}

impl TetrisView {
    fn new() -> Self {
        let board_width = CELL_SIZE * BOARD_COLS;
        let board_height = CELL_SIZE * BOARD_ROWS;
        let panel_width = WINDOW_WIDTH - board_width - (PADDING * 2.0) - GAP;

        Self {
            board_width,
            board_height,
            panel_width,
        }
    }
}

impl Render for TetrisView {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x101010))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .child(
                        div()
                            .w(px(self.board_width))
                            .h(px(self.board_height))
                            .bg(rgb(0x1c1c1c))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e)),
                    )
                    .child(
                        div()
                            .w(px(self.panel_width))
                            .h(px(self.board_height))
                            .bg(rgb(0x151515))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e)),
                    ),
            )
    }
}
