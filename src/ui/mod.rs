use gpui::{
    actions, div, px, rgb, size, Action, App, Application, Bounds, Context, Entity, FocusHandle,
    IntoElement, KeyBinding, KeyDownEvent, KeyUpEvent, Menu, MenuItem, MouseButton, Render,
    SystemMenuType, Window, WindowBounds, WindowOptions, prelude::*,
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
const BASE_WINDOW_WIDTH: f32 = WINDOW_WIDTH;
const BASE_WINDOW_HEIGHT: f32 = WINDOW_HEIGHT;
const BASE_CELL_SIZE: f32 = CELL_SIZE;

const BOARD_COLS: f32 = 10.0;
const BOARD_ROWS: f32 = 20.0;
const BASE_PADDING: f32 = 16.0;
const BASE_GAP: f32 = 16.0;
const DEFAULT_SFX_VOLUME: f32 = 0.7;
const SFX_VOLUME_STEP: f32 = 0.1;
const MIN_SCALE: f32 = 0.6;
const BASE_PANEL_TEXT: f32 = 12.0;
const BASE_TITLE_TEXT: f32 = 24.0;
const BASE_HINT_TEXT: f32 = 14.0;

actions!(
    tetris,
    [
        Quit,
        ToggleFullscreen,
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
            is_resizable: true,
            window_min_size: Some(size(
                px(BASE_WINDOW_WIDTH * MIN_SCALE),
                px(BASE_WINDOW_HEIGHT * MIN_SCALE),
            )),
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
        let window_handle = window.clone();

        cx.on_action({
            let window = window_handle.clone();
            move |_: &ToggleFullscreen, cx| {
                let _ = window.update(cx, |_, window, _| window.toggle_fullscreen());
            }
        });
        cx.bind_keys([KeyBinding::new("cmd-ctrl-f", ToggleFullscreen, None)]);
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
    last_action: Option<GameAction>,
    state: GameState,
    last_tick: Option<Instant>,
    focus_handle: FocusHandle,
    repeat_config: RepeatConfig,
    left_repeat: RepeatState,
    right_repeat: RepeatState,
    last_dir: Option<Direction>,
    audio: Option<AudioEngine>,
    started: bool,
    show_settings: bool,
    sfx_volume: f32,
    sfx_muted: bool,
    was_focused: bool,
}

impl TetrisView {
    fn new(cx: &mut Context<Self>, audio: Option<AudioEngine>) -> Self {
        let state = GameState::new(1, GameConfig::default());
        let focus_handle = cx.focus_handle();
        let mut view = Self {
            last_action: None,
            state,
            last_tick: None,
            focus_handle,
            repeat_config: RepeatConfig::default(),
            left_repeat: RepeatState::new(),
            right_repeat: RepeatState::new(),
            last_dir: None,
            audio,
            started: false,
            show_settings: false,
            sfx_volume: DEFAULT_SFX_VOLUME,
            sfx_muted: false,
            was_focused: true,
        };
        view.apply_audio_volume();

        view
    }
}

impl Render for TetrisView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = compute_scale(window);
        let cell_size = BASE_CELL_SIZE * scale;
        let padding = BASE_PADDING * scale;
        let gap = BASE_GAP * scale;
        let board_width = cell_size * BOARD_COLS;
        let board_height = cell_size * BOARD_ROWS;
        let panel_width =
            (BASE_WINDOW_WIDTH * scale) - board_width - (padding * 2.0) - gap;
        let now = Instant::now();
        let focused = self.focus_handle.is_focused(window);
        if self.was_focused && !focused {
            self.handle_focus_lost();
        }
        self.was_focused = focused;

        if let Some(prev) = self.last_tick {
            let elapsed_ms = now.duration_since(prev).as_millis() as u64;
            if elapsed_ms > 0 && self.started && !self.show_settings {
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

                row = row.child(render_cell(cell_kind, is_ghost, is_flash, cell_size));
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
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_4()
                    .child(
                        div()
                            .w(px(board_width))
                            .h(px(board_height))
                            .bg(rgb(0x1c1c1c))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e))
                            .relative()
                            .child(div().flex().flex_col().children(rows))
                            .child(render_line_clear_flash(self.state.line_clear_timer_ms > 0))
                            .child(render_lock_warning(self.state.lock_warning_intensity()))
                            .child(render_game_over_tint(self.state.game_over))
                            .child(render_overlay(
                                self.started,
                                self.show_settings,
                                self.state.paused,
                                self.state.game_over,
                                focused,
                                self.sfx_volume_label(),
                                self.sfx_muted,
                                scale,
                            )),
                    )
                    .child(
                        div()
                            .w(px(panel_width.max(cell_size * 4.0)))
                            .h(px(board_height))
                            .bg(rgb(0x1a1a1a))
                            .border(px(1.0))
                            .border_color(rgb(0x2e2e2e))
                            .p(px(padding * 0.75))
                            .flex()
                            .flex_col()
                            .gap(px(gap * 0.6))
                            .text_size(px(BASE_PANEL_TEXT * scale))
                            .text_color(rgb(0xe6e6e6))
                            .child(
                                div()
                                    .text_size(px(BASE_PANEL_TEXT * scale * 0.95))
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
                                    .gap(px(gap * 0.2))
                                    .child(format!("Score: {}", self.state.score))
                                    .child(format!("Level: {}", self.state.level))
                                    .child(format!("Lines: {}", self.state.lines))
                                    .child(format!(
                                        "Status: {}",
                                        self.status_label()
                                    ))
                                    .child(format!("Rules: {}", self.ruleset_label()))
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
                                    .child(format!("SFX: {}", self.sfx_volume_label()))
                                    .child(if self.state.is_classic_ruleset() {
                                        div().hidden()
                                    } else {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
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
                                            ))
                                            .child(if self.state.back_to_back {
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0xfacc15))
                                                    .child("B2B bonus active")
                                            } else {
                                                div().hidden()
                                            })
                                    })
                                    .child(render_lock_bar(
                                        self.state.lock_timer_ms,
                                        self.state.lock_delay_ms,
                                        self.state.is_grounded(),
                                        scale,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(gap * 0.2))
                                    .child(div().text_size(px(BASE_PANEL_TEXT * scale * 0.95)).child("Hold"))
                                    .child(render_preview(self.state.hold.as_ref(), cell_size)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(gap * 0.2))
                                    .child(div().text_size(px(BASE_PANEL_TEXT * scale * 0.95)).child("Next"))
                                    .child(render_preview(self.state.next_queue.first(), cell_size)),
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

fn render_cell(
    kind: Option<TetrominoType>,
    ghost: bool,
    flash: bool,
    cell_size: f32,
) -> impl IntoElement {
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
        .w(px(cell_size))
        .h(px(cell_size))
        .bg(fill)
        .border(px(1.0))
        .border_color(border)
}

fn render_preview(kind: Option<&TetrominoType>, cell_size: f32) -> impl IntoElement {
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
            row = row.child(render_preview_cell(cell_kind, cell_size));
        }
        rows.push(row);
    }

    div()
        .bg(rgb(0x101010))
        .border(px(1.0))
        .border_color(rgb(0x2a2a2a))
        .child(div().flex().flex_col().children(rows))
}

fn render_preview_cell(kind: Option<TetrominoType>, cell_size: f32) -> impl IntoElement {
    let size = cell_size * 0.6;
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
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" | "return" => {
                if !self.started {
                    self.start_game();
                }
            }
            "s" => {
                self.toggle_settings();
            }
            "m" => {
                self.toggle_mute();
            }
            "-" => {
                self.adjust_volume(-SFX_VOLUME_STEP);
            }
            "=" | "+" => {
                self.adjust_volume(SFX_VOLUME_STEP);
            }
            "0" => {
                self.reset_settings();
            }
            "escape" => {
                if self.show_settings {
                    self.show_settings = false;
                }
            }
            "left" => {
                if !self.can_accept_game_input() {
                    return;
                }
                if self.left_repeat.press() {
                    self.handle_action(GameAction::MoveLeft);
                    self.last_action = Some(GameAction::MoveLeft);
                }
                self.last_dir = Some(Direction::Left);
            }
            "right" => {
                if !self.can_accept_game_input() {
                    return;
                }
                if self.right_repeat.press() {
                    self.handle_action(GameAction::MoveRight);
                    self.last_action = Some(GameAction::MoveRight);
                }
                self.last_dir = Some(Direction::Right);
            }
            _ => {}
        }

        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window);
        }
    }

    fn on_key_up(&mut self, event: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" => self.left_repeat.release(),
            "right" => self.right_repeat.release(),
            _ => {}
        }
    }

    fn on_mouse_down(
        &mut self,
        _event: &gpui::MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
    }

    fn apply_repeats(&mut self, elapsed_ms: u64) {
        if !self.can_accept_game_input() {
            self.left_repeat.release();
            self.right_repeat.release();
            self.last_dir = None;
            return;
        }
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
                    self.handle_action(GameAction::MoveLeft);
                }
                if count > 0 {
                    self.last_action = Some(GameAction::MoveLeft);
                }
            }
            Some(Direction::Right) => {
                let count = self.right_repeat.tick(elapsed_ms, &self.repeat_config);
                for _ in 0..count {
                    self.handle_action(GameAction::MoveRight);
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

    fn handle_action(&mut self, action: GameAction) {
        if !self.started {
            if matches!(action, GameAction::Restart | GameAction::HardDrop) {
                self.start_game();
            }
            return;
        }
        if self.show_settings {
            return;
        }

        self.state.apply_action(action);
        if action == GameAction::Restart {
            self.started = true;
        }
    }

    fn start_game(&mut self) {
        self.started = true;
        self.show_settings = false;
        self.state.reset();
        self.state.paused = false;
    }

    fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings && !self.state.game_over {
            self.state.paused = true;
        }
    }

    fn toggle_mute(&mut self) {
        self.sfx_muted = !self.sfx_muted;
        self.apply_audio_volume();
    }

    fn adjust_volume(&mut self, delta: f32) {
        if self.sfx_muted {
            self.sfx_muted = false;
        }
        self.sfx_volume = (self.sfx_volume + delta).clamp(0.0, 1.0);
        self.apply_audio_volume();
    }

    fn reset_settings(&mut self) {
        self.sfx_muted = false;
        self.sfx_volume = DEFAULT_SFX_VOLUME;
        self.apply_audio_volume();
    }

    fn apply_audio_volume(&mut self) {
        if let Some(audio) = &self.audio {
            let volume = if self.sfx_muted { 0.0 } else { self.sfx_volume };
            audio.set_master_gain(volume);
        }
    }

    fn can_accept_game_input(&self) -> bool {
        self.started && !self.show_settings && !self.state.paused && !self.state.game_over
    }

    fn handle_focus_lost(&mut self) {
        self.left_repeat.release();
        self.right_repeat.release();
        self.last_dir = None;
        if self.started && !self.state.game_over {
            self.state.paused = true;
        }
    }

    fn status_label(&self) -> &'static str {
        if !self.started {
            "Title"
        } else if self.state.game_over {
            "Game Over"
        } else if self.show_settings {
            "Settings"
        } else if self.state.paused {
            "Paused"
        } else {
            "Playing"
        }
    }

    fn ruleset_label(&self) -> &'static str {
        if self.state.is_classic_ruleset() {
            "Classic"
        } else {
            "Modern"
        }
    }

    fn sfx_volume_label(&self) -> String {
        if self.sfx_muted {
            "Muted".to_string()
        } else {
            format!("{:.0}%", self.sfx_volume * 100.0)
        }
    }
}
fn render_overlay(
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
            .child(
                div()
                    .text_size(px(hint_size))
                    .child(format!("SFX Volume: {}{}", sfx_label, if muted { " (M)" } else { "" })),
            )
            .child(div().text_size(px(hint_size)).child("M: mute · +/-: volume · 0: reset"))
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
            .child(div().text_size(px(hint_size)).child("Press Enter or Space to Start"))
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

fn render_lock_bar(
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
        .child(div().text_size(px(BASE_PANEL_TEXT * scale * 0.95)).child("Lock delay"))
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

fn compute_scale(window: &Window) -> f32 {
    let viewport = window.viewport_size();
    let width = (viewport.width / px(1.0)).max(1.0);
    let height = (viewport.height / px(1.0)).max(1.0);
    let scale = (width / BASE_WINDOW_WIDTH).min(height / BASE_WINDOW_HEIGHT);
    scale.clamp(MIN_SCALE, 4.0)
}
fn register_action<A: Action + 'static>(
    cx: &mut App,
    view: Entity<TetrisView>,
    action: GameAction,
) {
    cx.on_action(move |_: &A, cx| {
        view.update(cx, |view, cx| {
            view.last_action = Some(action);
            view.handle_action(action);
            cx.notify();
        });
    });
}
