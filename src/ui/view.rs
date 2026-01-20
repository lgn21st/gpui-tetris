use gpui::{
    Context, FocusHandle, IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, Render, Window, div,
    prelude::*, px, rgb,
};
use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::{GameConfig, GameState};
use std::time::Instant;

use crate::ui::input::{InputAction, InputState};
use crate::ui::render::{render_board, render_panel};
use crate::ui::style::{
    BASE_CELL_SIZE, BASE_GAP, BASE_PADDING, BASE_WINDOW_HEIGHT, BASE_WINDOW_WIDTH, BOARD_COLS,
    BOARD_ROWS, MIN_SCALE, SFX_VOLUME_STEP,
};
use crate::ui::ui_state::UiState;

pub struct TetrisView {
    ui: UiState,
    last_tick: Option<Instant>,
    focus_handle: FocusHandle,
    input: InputState,
    was_focused: bool,
}

impl TetrisView {
    pub fn new(cx: &mut Context<Self>, audio: Option<AudioEngine>) -> Self {
        let state = GameState::new(1, GameConfig::default());
        let focus_handle = cx.focus_handle();
        let view = Self {
            ui: UiState::new(state, audio),
            last_tick: None,
            focus_handle,
            input: InputState::new(),
            was_focused: true,
        };
        view
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    pub fn receive_action(&mut self, action: GameAction) {
        self.ui.receive_action(action);
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
        let panel_width = (BASE_WINDOW_WIDTH * scale) - board_width - (padding * 2.0) - gap;
        let now = Instant::now();
        let focused = self.focus_handle.is_focused(window);
        if self.was_focused && !focused {
            self.handle_focus_lost();
        }
        self.was_focused = focused;
        let controller_actions = self.input.poll_controller();
        self.apply_input_actions(controller_actions);

        if let Some(prev) = self.last_tick {
            let elapsed_ms = now.duration_since(prev).as_millis() as u64;
            if elapsed_ms > 0 && self.ui.started && !self.ui.show_settings {
                self.ui.state.tick(elapsed_ms, false);
                let repeat_actions = self
                    .input
                    .apply_repeats(elapsed_ms, self.ui.can_accept_game_input());
                self.apply_input_actions(repeat_actions);
            }
        }
        self.last_tick = Some(now);
        window.request_animation_frame();
        self.play_sound_events();

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
                    .child(render_board(
                        &self.ui,
                        cell_size,
                        board_width,
                        board_height,
                        focused,
                        scale,
                    ))
                    .child(render_panel(
                        &self.ui,
                        cell_size,
                        board_height,
                        panel_width,
                        padding,
                        gap,
                        scale,
                    )),
            )
    }
}

impl TetrisView {
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" | "return" => {
                if !self.ui.started {
                    self.ui.start_game();
                }
            }
            "s" => {
                self.ui.toggle_settings();
            }
            "m" => {
                self.ui.toggle_mute();
            }
            "-" => {
                self.ui.adjust_volume(-SFX_VOLUME_STEP);
            }
            "=" | "+" => {
                self.ui.adjust_volume(SFX_VOLUME_STEP);
            }
            "0" => {
                self.ui.reset_settings();
            }
            "escape" => {
                if self.ui.show_settings {
                    self.ui.show_settings = false;
                }
            }
            "left" => {
                if !self.ui.can_accept_game_input() {
                    return;
                }
                let actions = self.input.set_keyboard_left(true);
                self.apply_input_actions(actions);
            }
            "right" => {
                if !self.ui.can_accept_game_input() {
                    return;
                }
                let actions = self.input.set_keyboard_right(true);
                self.apply_input_actions(actions);
            }
            _ => {}
        }

        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window);
        }
    }

    fn on_key_up(&mut self, event: &KeyUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" => {
                let actions = self.input.set_keyboard_left(false);
                self.apply_input_actions(actions);
            }
            "right" => {
                let actions = self.input.set_keyboard_right(false);
                self.apply_input_actions(actions);
            }
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

    fn apply_input_actions(&mut self, actions: Vec<InputAction>) {
        for entry in actions {
            self.ui.handle_action(entry.action);
            if entry.record {
                self.ui.last_action = Some(entry.action);
            }
        }
    }

    fn play_sound_events(&mut self) {
        let events = self.ui.state.take_sound_events();
        if let Some(audio) = &self.ui.audio {
            for event in events {
                audio.play(event);
            }
        }
    }

    fn handle_focus_lost(&mut self) {
        self.input.clear_focus_state();
        if self.ui.started && !self.ui.state.game_over {
            self.ui.state.paused = true;
        }
    }
}

fn compute_scale(window: &Window) -> f32 {
    let viewport = window.viewport_size();
    let width = (viewport.width / px(1.0)).max(1.0);
    let height = (viewport.height / px(1.0)).max(1.0);
    let scale = (width / BASE_WINDOW_WIDTH).min(height / BASE_WINDOW_HEIGHT);
    scale.clamp(MIN_SCALE, 4.0)
}
