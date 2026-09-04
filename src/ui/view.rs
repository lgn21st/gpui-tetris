use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::{
    Context, FocusHandle, IntoElement, MouseButton, Render, Subscription, Task, Window, div,
    linear_color_stop, linear_gradient, prelude::*, px,
};
use gpui_tetris::adapter::SocketAdapter;
use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::{GameConfig, GameState};
use std::time::{Duration, Instant};

use crate::ui::input::{InputAction, InputState};
use crate::ui::render::{
    OverlayState, RenderLayout, render_board, render_overlay, render_panel, theme,
};
use crate::ui::style::{BASE_PADDING, GROUP_CORNER_RADIUS, MIN_SCALE, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::ui::ui_state::UiState;

mod events;
mod settings;

pub struct TetrisView {
    ui: UiState,
    runtime_task: Option<Task<()>>,
    activation_subscription: Option<Subscription>,
    focus_initialized: bool,
    focus_handle: FocusHandle,
    input: InputState,
    was_focused: bool,
    input_actions: Vec<InputAction>,
    volume_slider: gpui_kit::Entity<SliderState>,
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
        let ui = UiState::with_runtime(gpui_tetris::runtime::Runtime::new(state, adapter), audio);
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(crate::ui::style::SFX_VOLUME_STEP)
                .default_value(ui.sfx_volume)
        });
        let volume_subscription =
            cx.subscribe(&volume_slider, |view, _, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event else {
                    return;
                };
                view.ui.set_volume(value.end());
                cx.notify();
            });
        Self {
            ui,
            runtime_task: None,
            activation_subscription: None,
            focus_initialized: false,
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
        if !self.focus_initialized {
            self.focus_initialized = true;
            self.activation_subscription =
                Some(cx.observe_window_activation(window, |view, window, _| {
                    view.update_focus(window);
                }));
            self.focus_handle.focus(window, cx);
            self.runtime_task = Some(cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(8))
                        .await;
                    let result = cx.update(|window, cx| {
                        view.update(cx, |view, cx| {
                            view.update_focus(window);
                            view.advance_runtime(Instant::now());
                            // Kit's root can retain its rendered subtree until the window is dirty.
                            window.refresh();
                            cx.notify();
                        })
                    });
                    if !matches!(result, Ok(Ok(()))) {
                        break;
                    }
                }
            }));
        }
        if self.settings_slider_needs_sync {
            self.sync_volume_slider(window, cx);
            self.settings_slider_needs_sync = false;
        }
        let scale = compute_scale(window);
        let layout = RenderLayout::new(scale);
        let now = Instant::now();
        self.update_focus(window);

        self.ui.sync_panel_labels();

        let board = render_board(&mut self.ui, &layout, now);
        let panel = render_panel(&mut self.ui, &layout);
        let overlay = render_overlay(&OverlayState {
            started: self.ui.runtime.started(),
            show_settings: self.ui.show_settings,
            paused: self.ui.runtime.state().paused,
            game_over: self.ui.runtime.state().game_over,
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
        let events = self.ui.runtime.drain_sounds();
        if let Some(audio) = &self.ui.audio {
            for event in events {
                audio.play(event);
            }
        }
    }

    fn update_focus(&mut self, window: &Window) -> bool {
        let focused = window.is_window_active() && self.focus_handle.is_focused(window);
        if self.was_focused && !focused {
            self.handle_focus_lost();
        }
        self.was_focused = focused;
        focused
    }

    fn advance_runtime(&mut self, now: Instant) {
        self.input
            .poll_controller_into(self.was_focused, &mut self.input_actions);
        self.apply_buffered_actions();
        let active = self.was_focused;
        let input = &mut self.input;
        self.ui.runtime.pump(now, |ms, playable, out| {
            input.apply_repeats_into(ms, active && playable, out);
        });
        self.ui.sync_lifecycle();
        if !self.ui.can_accept_game_input() {
            self.input.clear_focus_state();
        }
        self.ui.update_active_animation(now);
        self.play_sound_events();
    }
}

fn compute_scale(window: &Window) -> f32 {
    let viewport = window.viewport_size();
    let width = (viewport.width / px(1.0)).max(1.0);
    let height = (viewport.height / px(1.0)).max(1.0);
    let scale = (width / WINDOW_WIDTH).min(height / WINDOW_HEIGHT);
    scale.clamp(MIN_SCALE, 4.0)
}
