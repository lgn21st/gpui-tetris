use gpui::{
    actions, div, px, rgb, size, Action, App, Application, Bounds, Context, Entity, IntoElement,
    KeyBinding, Menu, MenuItem, Render, SystemMenuType, Window, WindowBounds, WindowOptions,
    prelude::*,
};

use gpui_tetris::game::input::GameAction;

pub const WINDOW_WIDTH: f32 = 480.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const CELL_SIZE: f32 = 24.0;

const BOARD_COLS: f32 = 10.0;
const BOARD_ROWS: f32 = 20.0;
const PADDING: f32 = 16.0;
const GAP: f32 = 16.0;

actions!(
    tetris,
    [
        Quit,
        MoveLeft,
        MoveRight,
        SoftDrop,
        HardDrop,
        RotateCw,
        RotateCcw
    ]
);

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            is_resizable: false,
            ..Default::default()
        };

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("left", MoveLeft, None),
            KeyBinding::new("right", MoveRight, None),
            KeyBinding::new("down", SoftDrop, None),
            KeyBinding::new("up", RotateCw, None),
            KeyBinding::new("space", HardDrop, None),
        ]);
        cx.set_menus(vec![Menu {
            name: "gpui-tetris".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        }]);

        let window = cx
            .open_window(options, |_, cx| cx.new(|_| TetrisView::new()))
            .unwrap();
        let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();

        register_action::<MoveLeft>(cx, view.clone(), GameAction::MoveLeft);
        register_action::<MoveRight>(cx, view.clone(), GameAction::MoveRight);
        register_action::<SoftDrop>(cx, view.clone(), GameAction::SoftDrop);
        register_action::<HardDrop>(cx, view.clone(), GameAction::HardDrop);
        register_action::<RotateCw>(cx, view.clone(), GameAction::RotateCw);
        register_action::<RotateCcw>(cx, view, GameAction::RotateCcw);

        cx.activate(true);
    })
}

struct TetrisView {
    board_width: f32,
    board_height: f32,
    panel_width: f32,
    last_action: Option<GameAction>,
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
            last_action: None,
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
                            .border_color(rgb(0x2e2e2e))
                            .p_2()
                            .child(format!(
                                "Last input: {}",
                                self.last_action
                                    .as_ref()
                                    .map(action_label)
                                    .unwrap_or("None")
                            )),
                    ),
            )
    }
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

fn register_action<A: Action + 'static>(
    cx: &mut App,
    view: Entity<TetrisView>,
    action: GameAction,
) {
    cx.on_action(move |_: &A, cx| {
        view.update(cx, |view, cx| {
            view.last_action = Some(action);
            cx.notify();
        });
    });
}
