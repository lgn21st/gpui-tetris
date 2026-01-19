use gpui::{
    actions, div, px, rgb, size, Action, App, Application, Bounds, Context, Entity, IntoElement,
    KeyBinding, Menu, MenuItem, Render, SystemMenuType, Window, WindowBounds, WindowOptions,
    prelude::*,
};

use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::TetrominoType;
use gpui_tetris::game::state::{GameConfig, GameState};

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
        RotateCcw,
        Pause,
        Restart
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
            KeyBinding::new("p", Pause, None),
            KeyBinding::new("r", Restart, None),
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
        register_action::<RotateCcw>(cx, view.clone(), GameAction::RotateCcw);
        register_action::<Pause>(cx, view.clone(), GameAction::Pause);
        register_action::<Restart>(cx, view, GameAction::Restart);

        cx.activate(true);
    })
}

struct TetrisView {
    board_width: f32,
    board_height: f32,
    panel_width: f32,
    last_action: Option<GameAction>,
    state: GameState,
}

impl TetrisView {
    fn new() -> Self {
        let board_width = CELL_SIZE * BOARD_COLS;
        let board_height = CELL_SIZE * BOARD_ROWS;
        let panel_width = WINDOW_WIDTH - board_width - (PADDING * 2.0) - GAP;
        let state = GameState::new(1, GameConfig::default());

        Self {
            board_width,
            board_height,
            panel_width,
            last_action: None,
            state,
        }
    }
}

impl Render for TetrisView {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let active_blocks = self.state.active.blocks(self.state.active.rotation);
        let mut active_cells = Vec::with_capacity(4);
        for (dx, dy) in active_blocks.iter() {
            active_cells.push((self.state.active.x + dx, self.state.active.y + dy));
        }
        let ghost_cells = self.state.ghost_blocks();

        let mut rows = Vec::new();
        for y in 0..BOARD_ROWS as i32 {
            let mut row = div().flex();
            for x in 0..BOARD_COLS as i32 {
                let mut cell_kind = self.state.board.cells[y as usize][x as usize].kind;
                let mut is_ghost = false;
                if active_cells.iter().any(|(ax, ay)| *ax == x && *ay == y) {
                    cell_kind = Some(self.state.active.kind);
                } else if ghost_cells.iter().any(|(gx, gy)| *gx == x && *gy == y) {
                    cell_kind = Some(self.state.active.kind);
                    is_ghost = true;
                }

                row = row.child(render_cell(cell_kind, is_ghost));
            }
            rows.push(row);
        }

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
                            .border_color(rgb(0x2e2e2e))
                            .child(div().flex().flex_col().children(rows)),
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
                            ))
                            .child(format!("Score: {}", self.state.score))
                            .child(format!("Level: {}", self.state.level))
                            .child(format!("Lines: {}", self.state.lines))
                            .child(format!(
                                "Paused: {}",
                                if self.state.paused { "Yes" } else { "No" }
                            ))
                            .child(format!(
                                "Status: {}",
                                if self.state.game_over {
                                    "Game Over"
                                } else {
                                    "Playing"
                                }
                            ))
                            .child(format!(
                                "Hold: {}",
                                self.state
                                    .hold
                                    .as_ref()
                                    .map(piece_label)
                                    .unwrap_or("None")
                            ))
                            .child(format!(
                                "Next: {}",
                                self.state
                                    .next_queue
                                    .first()
                                    .map(piece_label)
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

fn piece_label(kind: &TetrominoType) -> &'static str {
    match kind {
        TetrominoType::I => "I",
        TetrominoType::O => "O",
        TetrominoType::T => "T",
        TetrominoType::S => "S",
        TetrominoType::Z => "Z",
        TetrominoType::J => "J",
        TetrominoType::L => "L",
    }
}

fn render_cell(kind: Option<TetrominoType>, ghost: bool) -> impl IntoElement {
    let color = match kind {
        Some(TetrominoType::I) => rgb(0x4fd1c5),
        Some(TetrominoType::O) => rgb(0xf6e05e),
        Some(TetrominoType::T) => rgb(0x9f7aea),
        Some(TetrominoType::S) => rgb(0x68d391),
        Some(TetrominoType::Z) => rgb(0xfc8181),
        Some(TetrominoType::J) => rgb(0x63b3ed),
        Some(TetrominoType::L) => rgb(0xf6ad55),
        None => rgb(0x101010),
    };
    let fill = if ghost {
        rgb(0x2a2a2a)
    } else {
        color
    };

    div()
        .w(px(CELL_SIZE))
        .h(px(CELL_SIZE))
        .bg(fill)
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
}
fn register_action<A: Action + 'static>(
    cx: &mut App,
    view: Entity<TetrisView>,
    action: GameAction,
) {
    cx.on_action(move |_: &A, cx| {
        view.update(cx, |view, cx| {
            view.last_action = Some(action);
            view.state.apply_action(action);
            cx.notify();
        });
    });
}
