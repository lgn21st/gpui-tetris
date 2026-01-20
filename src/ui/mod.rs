use gpui::{
    actions, div, px, rgb, size, Action, App, Application, Bounds, Context, Entity, FocusHandle,
    IntoElement, KeyBinding, KeyDownEvent, KeyUpEvent, Menu, MenuItem, Render, SystemMenuType,
    Window, WindowBounds, WindowOptions, prelude::*,
};

use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::{GameAction, RepeatConfig, RepeatState};
use gpui_tetris::game::pieces::{Tetromino, TetrominoType};
use gpui_tetris::game::state::{GameConfig, GameState};
use std::path::Path;
use std::time::Instant;

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
        Hold,
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
        let audio_engine = match AudioEngine::new(Path::new("assets/sfx")) {
            Ok(engine) => Some(engine),
            Err(err) => {
                eprintln!("audio disabled: {err}");
                None
            }
        };

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("down", SoftDrop, None),
            KeyBinding::new("up", RotateCw, None),
            KeyBinding::new("space", HardDrop, None),
            KeyBinding::new("c", Hold, None),
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
            .open_window(options, move |_, cx| {
                let audio = audio_engine.clone();
                cx.new(|cx| TetrisView::new(cx, audio))
            })
            .unwrap();
        let view = window.update(cx, |_, _, cx| cx.entity()).unwrap();

        register_action::<MoveLeft>(cx, view.clone(), GameAction::MoveLeft);
        register_action::<MoveRight>(cx, view.clone(), GameAction::MoveRight);
        register_action::<SoftDrop>(cx, view.clone(), GameAction::SoftDrop);
        register_action::<HardDrop>(cx, view.clone(), GameAction::HardDrop);
        register_action::<RotateCw>(cx, view.clone(), GameAction::RotateCw);
        register_action::<RotateCcw>(cx, view.clone(), GameAction::RotateCcw);
        register_action::<Hold>(cx, view.clone(), GameAction::Hold);
        register_action::<Pause>(cx, view.clone(), GameAction::Pause);
        register_action::<Restart>(cx, view, GameAction::Restart);

        window
            .update(cx, |view, window, _| {
                window.focus(&view.focus_handle);
            })
            .unwrap();
        cx.activate(true);
    })
}

struct TetrisView {
    board_width: f32,
    board_height: f32,
    panel_width: f32,
    last_action: Option<GameAction>,
    state: GameState,
    last_tick: Option<Instant>,
    focus_handle: FocusHandle,
    repeat_config: RepeatConfig,
    left_repeat: RepeatState,
    right_repeat: RepeatState,
    last_dir: Option<Direction>,
    audio: Option<AudioEngine>,
}

impl TetrisView {
    fn new(cx: &mut Context<Self>, audio: Option<AudioEngine>) -> Self {
        let board_width = CELL_SIZE * BOARD_COLS;
        let board_height = CELL_SIZE * BOARD_ROWS;
        let panel_width = WINDOW_WIDTH - board_width - (PADDING * 2.0) - GAP;
        let state = GameState::new(1, GameConfig::default());
        let focus_handle = cx.focus_handle();

        Self {
            board_width,
            board_height,
            panel_width,
            last_action: None,
            state,
            last_tick: None,
            focus_handle,
            repeat_config: RepeatConfig::default(),
            left_repeat: RepeatState::new(),
            right_repeat: RepeatState::new(),
            last_dir: None,
            audio,
        }
    }
}

impl Render for TetrisView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        if let Some(prev) = self.last_tick {
            let elapsed_ms = now.duration_since(prev).as_millis() as u64;
            if elapsed_ms > 0 {
                self.state.tick(elapsed_ms, false);
                self.apply_repeats(elapsed_ms);
            }
        }
        self.last_tick = Some(now);
        window.request_animation_frame();
        self.play_sound_events();

        let show_active = !self.state.is_line_clear_active();
        let mut active_cells = Vec::new();
        let landing_cells = if self.state.landing_flash_active() {
            self.state.last_lock_cells
        } else {
            [(0, 0); 4]
        };
        let ghost_cells = if show_active {
            let active_blocks = self.state.active.blocks(self.state.active.rotation);
            active_cells = Vec::with_capacity(4);
            for (dx, dy) in active_blocks.iter() {
                active_cells.push((self.state.active.x + dx, self.state.active.y + dy));
            }
            self.state.ghost_blocks()
        } else {
            [(0, 0); 4]
        };

        let mut rows = Vec::new();
        for y in 0..BOARD_ROWS as i32 {
            let mut row = div().flex();
            for x in 0..BOARD_COLS as i32 {
                let mut cell_kind = self.state.board.cells[y as usize][x as usize].kind;
                let mut is_ghost = false;
                let mut is_flash = false;
                if self.state.landing_flash_active()
                    && landing_cells.iter().any(|(lx, ly)| *lx == x && *ly == y)
                {
                    is_flash = true;
                }
                if show_active && active_cells.iter().any(|(ax, ay)| *ax == x && *ay == y) {
                    cell_kind = Some(self.state.active.kind);
                } else if show_active && ghost_cells.iter().any(|(gx, gy)| *gx == x && *gy == y)
                {
                    cell_kind = Some(self.state.active.kind);
                    is_ghost = true;
                }

                row = row.child(render_cell(cell_kind, is_ghost, is_flash));
            }
            rows.push(row);
        }

        div()
            .size_full()
            .bg(rgb(0x101010))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
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
                            .relative()
                            .child(div().flex().flex_col().children(rows))
                            .child(render_line_clear_flash(self.state.line_clear_timer_ms > 0))
                            .child(render_lock_warning(self.state.lock_warning_intensity()))
                            .child(render_game_over_tint(self.state.game_over))
                            .child(render_overlay(self.state.paused, self.state.game_over)),
                    )
                    .child(
                        div()
                            .w(px(self.panel_width))
                            .h(px(self.board_height))
                            .bg(rgb(0x1a1a1a))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .text_color(rgb(0xe6e6e6))
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!(
                                        "Last input: {}",
                                        self.last_action
                                            .as_ref()
                                            .map(action_label)
                                            .unwrap_or("None")
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(format!("Score: {}", self.state.score))
                                    .child(format!("Level: {}", self.state.level))
                                    .child(format!("Lines: {}", self.state.lines))
                                    .child(format!(
                                        "Status: {}",
                                        if self.state.game_over {
                                            "Game Over"
                                        } else if self.state.paused {
                                            "Paused"
                                        } else {
                                            "Playing"
                                        }
                                    ))
                                    .child(format!(
                                        "Hold: {}",
                                        if self.state.can_hold {
                                            "Ready"
                                        } else {
                                            "Used"
                                        }
                                    ))
                                    .child(format!(
                                        "Grounded: {}",
                                        if self.state.is_grounded() { "Yes" } else { "No" }
                                    ))
                                    .child(format!(
                                        "Lock resets: {}/{}",
                                        self.state.lock_reset_remaining(),
                                        self.state.lock_reset_limit
                                    ))
                                    .child(format!(
                                        "Combo: {}",
                                        if self.state.combo >= 0 {
                                            self.state.combo.to_string()
                                        } else {
                                            "-".to_string()
                                        }
                                    ))
                                    .child(format!(
                                        "B2B: {}",
                                        if self.state.back_to_back { "Yes" } else { "No" }
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_sm().child("Hold"))
                                    .child(render_preview(self.state.hold.as_ref())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_sm().child("Next"))
                                    .child(render_preview(self.state.next_queue.first())),
                            ),
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

fn render_cell(kind: Option<TetrominoType>, ghost: bool, flash: bool) -> impl IntoElement {
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
    let border = if flash { rgb(0xfef3c7) } else { rgb(0x2a2a2a) };

    div()
        .w(px(CELL_SIZE))
        .h(px(CELL_SIZE))
        .bg(fill)
        .border(px(1.0))
        .border_color(border)
}

fn render_preview(kind: Option<&TetrominoType>) -> impl IntoElement {
    const PREVIEW_SIZE: i32 = 4;
    let mut filled = [[false; PREVIEW_SIZE as usize]; PREVIEW_SIZE as usize];

    if let Some(kind) = kind {
        let piece = Tetromino::new(*kind, 0, 0);
        for (x, y) in piece.blocks(piece.rotation).iter() {
            let ux = *x as usize;
            let uy = *y as usize;
            if ux < PREVIEW_SIZE as usize && uy < PREVIEW_SIZE as usize {
                filled[uy][ux] = true;
            }
        }
    }

    let mut rows = Vec::new();
    for y in 0..PREVIEW_SIZE {
        let mut row = div().flex();
        for x in 0..PREVIEW_SIZE {
            let cell_kind = if filled[y as usize][x as usize] {
                kind.copied()
            } else {
                None
            };
            row = row.child(render_preview_cell(cell_kind));
        }
        rows.push(row);
    }

    div()
        .bg(rgb(0x101010))
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
        .child(div().flex().flex_col().children(rows))
}

fn render_preview_cell(kind: Option<TetrominoType>) -> impl IntoElement {
    let size = CELL_SIZE * 0.6;
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

    div()
        .w(px(size))
        .h(px(size))
        .bg(color)
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}

impl TetrisView {
    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" => {
                if self.left_repeat.press() {
                    self.state.apply_action(GameAction::MoveLeft);
                    self.last_action = Some(GameAction::MoveLeft);
                }
                self.last_dir = Some(Direction::Left);
            }
            "right" => {
                if self.right_repeat.press() {
                    self.state.apply_action(GameAction::MoveRight);
                    self.last_action = Some(GameAction::MoveRight);
                }
                self.last_dir = Some(Direction::Right);
            }
            _ => {}
        }
    }

    fn on_key_up(&mut self, event: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" => self.left_repeat.release(),
            "right" => self.right_repeat.release(),
            _ => {}
        }
    }

    fn apply_repeats(&mut self, elapsed_ms: u64) {
        let direction = match (self.left_repeat.is_held(), self.right_repeat.is_held()) {
            (true, false) => Some(Direction::Left),
            (false, true) => Some(Direction::Right),
            (true, true) => self.last_dir,
            _ => None,
        };

        match direction {
            Some(Direction::Left) => {
                let count = self.left_repeat.tick(elapsed_ms, &self.repeat_config);
                for _ in 0..count {
                    self.state.apply_action(GameAction::MoveLeft);
                }
                if count > 0 {
                    self.last_action = Some(GameAction::MoveLeft);
                }
            }
            Some(Direction::Right) => {
                let count = self.right_repeat.tick(elapsed_ms, &self.repeat_config);
                for _ in 0..count {
                    self.state.apply_action(GameAction::MoveRight);
                }
                if count > 0 {
                    self.last_action = Some(GameAction::MoveRight);
                }
            }
            None => {}
        }
    }

    fn play_sound_events(&mut self) {
        let events = self.state.take_sound_events();
        if let Some(audio) = &self.audio {
            for event in events {
                audio.play(event);
            }
        }
    }
}
fn render_overlay(paused: bool, game_over: bool) -> impl IntoElement {
    if !paused && !game_over {
        return div().hidden();
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
        .text_xl()
        .text_color(rgb(0xf5f5f5))
        .child(label)
        .child(div().text_sm().child(hint))
}

fn render_line_clear_flash(active: bool) -> impl IntoElement {
    if !active {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0xffffff))
        .opacity(0.12)
}

fn render_game_over_tint(active: bool) -> impl IntoElement {
    if !active {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0x3a0f0f))
        .opacity(0.28)
}

fn render_lock_warning(intensity: f32) -> impl IntoElement {
    if intensity <= 0.0 {
        return div().hidden();
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(rgb(0x7a1c1c))
        .opacity(intensity)
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
