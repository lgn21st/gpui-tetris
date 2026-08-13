use gpui::{
    Context, FocusHandle, IntoElement, MouseButton, Render, Subscription, Window, div,
    linear_color_stop, linear_gradient, prelude::*, px,
};
use gpui_component::slider::{SliderEvent, SliderState};
use gpui_tetris::adapter::SocketAdapter;
use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::{GameConfig, GameState};
use std::time::Instant;

use crate::ui::input::{InputAction, InputState};
use crate::ui::render::{
    OverlayState, RenderLayout, render_board, render_overlay, render_panel, theme,
};
use crate::ui::style::{BASE_PADDING, GROUP_CORNER_RADIUS, MIN_SCALE, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::ui::ui_state::UiState;

mod events;
mod settings;

const MAX_CATCH_UP_STEPS: u64 = 15;

pub struct TetrisView {
    ui: UiState,
    adapter: Option<SocketAdapter>,
    last_tick: Option<Instant>,
    tick_accumulator_ms: u64,
    focus_handle: FocusHandle,
    input: InputState,
    was_focused: bool,
    input_actions: Vec<InputAction>,
    volume_slider: gpui::Entity<SliderState>,
    settings_slider_needs_sync: bool,
    _volume_subscription: Subscription,
}

impl TetrisView {
    pub fn new(cx: &mut Context<Self>, audio: Option<AudioEngine>) -> Self {
        let state = GameState::new(1, GameConfig::default());
        let adapter = match SocketAdapter::from_env() {
            Ok(adapter) => adapter,
            Err(err) => {
                eprintln!("adapter disabled: {err}");
                None
            }
        };
        if let Some(adapter) = &adapter
            && let Some(addr) = adapter.local_addr()
        {
            eprintln!("adapter listening on tcp://{addr}");
        }
        let focus_handle = cx.focus_handle();
        let mut ui = UiState::new(state, audio);
        if adapter.is_some() {
            ui.start_game();
        }
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(crate::ui::style::SFX_VOLUME_STEP)
                .default_value(ui.sfx_volume)
        });
        let volume_subscription =
            cx.subscribe(&volume_slider, |view, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event;
                view.ui.set_volume(value.end());
                cx.notify();
            });
        Self {
            ui,
            adapter,
            last_tick: None,
            tick_accumulator_ms: 0,
            focus_handle,
            input: InputState::new(),
            was_focused: false,
            input_actions: Vec::with_capacity(16),
            volume_slider,
            settings_slider_needs_sync: false,
            _volume_subscription: volume_subscription,
        }
    }

    pub fn receive_action(&mut self, action: GameAction) {
        self.ui.receive_action(action);
    }

    pub fn toggle_settings(&mut self) {
        self.ui.toggle_settings();
        self.settings_slider_needs_sync = self.ui.show_settings;
    }
}

impl Render for TetrisView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.last_tick.is_none() {
            self.focus_handle.focus(window);
        }
        if self.settings_slider_needs_sync {
            self.sync_volume_slider(window, cx);
            self.settings_slider_needs_sync = false;
        }
        let scale = compute_scale(window);
        let layout = RenderLayout::new(scale);
        let now = Instant::now();
        self.update_focus(window);
        self.advance_frame(now);

        window.request_animation_frame();
        self.play_sound_events();
        self.ui.sync_panel_labels();

        let board = render_board(&mut self.ui, &layout, now);
        let panel = render_panel(&mut self.ui, &layout);
        let overlay = render_overlay(&OverlayState {
            started: self.ui.started,
            show_settings: self.ui.show_settings,
            paused: self.ui.state.paused,
            game_over: self.ui.state.game_over,
            scale,
        });
        let game_content = div()
            .w(px(layout.content_width))
            .h(px(layout.board_height))
            .relative()
            .rounded(px(GROUP_CORNER_RADIUS * scale))
            .bg(linear_gradient(
                135.0,
                linear_color_stop(theme::group_start(), 0.0),
                linear_color_stop(theme::group_end(), 1.0),
            ))
            .border(px(1.0))
            .border_color(theme::border())
            .shadow_lg()
            .child(div().flex().gap(px(layout.gap)).child(board).child(panel));

        div()
            .size_full()
            .relative()
            .bg(theme::app_bg())
            .p(px(BASE_PADDING * scale))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .child(game_content)
            .child(overlay)
            .child(self.render_settings_overlay(scale, cx))
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
        if let Some(adapter) = self.adapter.as_mut()
            && adapter.poll_and_apply(&mut self.ui.state)
        {
            self.ui.mark_game_dirty();
        }

        if let Some(prev) = self.last_tick {
            let elapsed_ms = now.duration_since(prev).as_millis() as u64;
            if elapsed_ms > 0 && self.ui.started && !self.ui.show_settings {
                let step_ms = self.ui.state.tick_ms.max(1);
                let elapsed_ms = bounded_elapsed_ms(elapsed_ms, step_ms);
                self.tick_accumulator_ms = self.tick_accumulator_ms.saturating_add(elapsed_ms);
                while self.tick_accumulator_ms >= step_ms {
                    self.ui.state.tick(step_ms, false);
                    self.ui.mark_game_dirty();
                    self.input.apply_repeats_into(
                        step_ms,
                        self.ui.can_accept_game_input(),
                        &mut self.input_actions,
                    );
                    self.apply_buffered_actions();
                    self.tick_accumulator_ms -= step_ms;
                }
            } else {
                self.tick_accumulator_ms = 0;
            }
        }
        self.ui.update_active_animation(now);
        if let Some(adapter) = self.adapter.as_mut() {
            adapter.emit_observation(&mut self.ui.state);
        }
        self.last_tick = Some(now);
    }
}

fn bounded_elapsed_ms(elapsed_ms: u64, step_ms: u64) -> u64 {
    elapsed_ms.min(step_ms.saturating_mul(MAX_CATCH_UP_STEPS))
}

fn compute_scale(window: &Window) -> f32 {
    let viewport = window.viewport_size();
    let width = (viewport.width / px(1.0)).max(1.0);
    let height = (viewport.height / px(1.0)).max(1.0);
    let scale = (width / WINDOW_WIDTH).min(height / WINDOW_HEIGHT);
    scale.clamp(MIN_SCALE, 4.0)
}

#[cfg(test)]
mod tests {
    use super::{MAX_CATCH_UP_STEPS, bounded_elapsed_ms};

    #[test]
    fn frame_delta_is_bounded_to_avoid_unbounded_catch_up() {
        assert_eq!(bounded_elapsed_ms(u64::MAX, 16), 16 * MAX_CATCH_UP_STEPS);
        assert_eq!(bounded_elapsed_ms(15, 16), 15);
    }
}
