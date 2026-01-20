use gpui::{Context, KeyDownEvent, KeyUpEvent, Window};
use crate::ui::input::InputAction;
use crate::ui::style::SFX_VOLUME_STEP;
use crate::ui::view::TetrisView;

impl TetrisView {
    pub(super) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
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

    pub(super) fn on_key_up(
        &mut self,
        event: &KeyUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
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

    pub(super) fn on_mouse_down(
        &mut self,
        _event: &gpui::MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window);
    }

    pub(super) fn apply_input_actions(&mut self, actions: Vec<InputAction>) {
        for entry in actions {
            self.ui.handle_action(entry.action);
            if entry.record {
                self.ui.last_action = Some(entry.action);
            }
        }
    }

    pub(super) fn handle_focus_lost(&mut self) {
        self.input.clear_focus_state();
        if self.ui.started && !self.ui.state.game_over {
            self.ui.state.paused = true;
        }
    }
}
