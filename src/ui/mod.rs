use gpui::{div, px, rgb, App, IntoElement, Render, ViewContext, WindowOptions};

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const CELL_SIZE: f32 = 24.0;

const BOARD_COLS: f32 = 10.0;
const BOARD_ROWS: f32 = 20.0;
const PADDING: f32 = 24.0;

pub fn run() {
    App::new().run(|cx| {
        let options = WindowOptions {
            resizable: false,
            ..Default::default()
        };

        cx.open_window(options, |cx| cx.new_view(|_cx| TetrisView::new()));
    });
}

struct TetrisView {
    board_width: f32,
    board_height: f32,
    panel_width: f32,
    top_margin: f32,
}

impl TetrisView {
    fn new() -> Self {
        let board_width = CELL_SIZE * BOARD_COLS;
        let board_height = CELL_SIZE * BOARD_ROWS;
        let panel_width = WINDOW_WIDTH - board_width - (PADDING * 2.0);
        let top_margin = (WINDOW_HEIGHT - board_height) / 2.0;

        Self {
            board_width,
            board_height,
            panel_width,
            top_margin,
        }
    }
}

impl Render for TetrisView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT))
            .bg(rgb(0x101010))
            .child(
                div()
                    .size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT))
                    .padding(px(PADDING))
                    .child(
                        div()
                            .size(px(self.board_width), px(self.board_height))
                            .bg(rgb(0x1c1c1c))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e))
                            .margin_top(px(self.top_margin)),
                    )
                    .child(
                        div()
                            .size(px(self.panel_width), px(self.board_height))
                            .bg(rgb(0x151515))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e))
                            .margin_top(px(self.top_margin))
                            .margin_left(px(PADDING)),
                    ),
            )
    }
}
