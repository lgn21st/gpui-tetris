use gpui::{
    Context, FocusHandle, IntoElement, MouseButton, Render, Window, div, prelude::*, px, rgb,
};
use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::{GameConfig, GameState};
use std::time::Instant;

use crate::ui::input::{InputAction, InputState};
use crate::ui::render::{RenderLayout, render_board, render_panel};
use crate::ui::style::{BASE_WINDOW_HEIGHT, BASE_WINDOW_WIDTH, MIN_SCALE};
use crate::ui::ui_state::UiState;

mod events;

pub struct TetrisView {
    ui: UiState,
    last_tick: Option<Instant>,
    focus_handle: FocusHandle,
    input: InputState,
    was_focused: bool,
    input_actions: Vec<InputAction>,
}

impl TetrisView {
    pub fn new(cx: &mut Context<Self>, audio: Option<AudioEngine>) -> Self {
        let state = GameState::new(1, GameConfig::default());
        let focus_handle = cx.focus_handle();
        Self {
            ui: UiState::new(state, audio),
            last_tick: None,
            focus_handle,
            input: InputState::new(),
            was_focused: true,
            input_actions: Vec::with_capacity(16),
        }
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
        let layout = RenderLayout::new(scale);
        let now = Instant::now();
        let focused = self.update_focus(window);
        self.advance_frame(now);

        window.request_animation_frame();
        self.play_sound_events();
        self.ui.sync_panel_labels();

        let board = render_board(&mut self.ui, &layout, focused);
        let panel = render_panel(&mut self.ui, &layout);

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
            .child(div().flex().gap_4().p_4().child(board).child(panel))
    }
}

impl TetrisView {
    fn play_sound_events(&mut self) {
        let events = self.ui.state.take_sound_events();
        if let Some(audio) = &self.ui.audio {
            for event in events {
                audio.play(event);
            }
        }
    }

    fn update_focus(&mut self, window: &Window) -> bool {
        let focused = self.focus_handle.is_focused(window);
        if self.was_focused && !focused {
            self.handle_focus_lost();
        }
        self.was_focused = focused;
        focused
    }

    fn advance_frame(&mut self, now: Instant) {
        self.input.poll_controller_into(&mut self.input_actions);
        self.apply_buffered_actions();

        if let Some(prev) = self.last_tick {
            let elapsed_ms = now.duration_since(prev).as_millis() as u64;
            if elapsed_ms > 0 && self.ui.started && !self.ui.show_settings {
                self.ui.state.tick(elapsed_ms, false);
                self.ui.mark_game_dirty();
                self.input.apply_repeats_into(
                    elapsed_ms,
                    self.ui.can_accept_game_input(),
                    &mut self.input_actions,
                );
                self.apply_buffered_actions();
            }
        }
        self.last_tick = Some(now);
    }
}

fn compute_scale(window: &Window) -> f32 {
    let viewport = window.viewport_size();
    let width = (viewport.width / px(1.0)).max(1.0);
    let height = (viewport.height / px(1.0)).max(1.0);
    let scale = (width / BASE_WINDOW_WIDTH).min(height / BASE_WINDOW_HEIGHT);
    scale.clamp(MIN_SCALE, 4.0)
}
