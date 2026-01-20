use crate::ui::input::InputAction;
use crate::ui::style::SFX_VOLUME_STEP;
use crate::ui::view::TetrisView;
use gpui::{Context, KeyDownEvent, KeyUpEvent, Window};

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
                self.ui.close_settings();
            }
            "left" => {
                if !self.ui.can_accept_game_input() {
                    return;
                }
                let actions = self.input.set_keyboard_left(true);
                self.apply_input_actions(&actions);
            }
            "right" => {
                if !self.ui.can_accept_game_input() {
                    return;
                }
                let actions = self.input.set_keyboard_right(true);
                self.apply_input_actions(&actions);
            }
            _ => {}
        }

        if event.keystroke.key.as_str() == "f"
            && event.keystroke.modifiers.control
            && event.keystroke.modifiers.platform
        {
            window.activate_window();
            window.toggle_fullscreen();
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
                self.apply_input_actions(&actions);
            }
            "right" => {
                let actions = self.input.set_keyboard_right(false);
                self.apply_input_actions(&actions);
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

    pub(super) fn apply_input_actions(&mut self, actions: &[InputAction]) {
        for entry in actions {
            self.ui.apply_action(entry.action, entry.record);
        }
    }

    pub(super) fn apply_buffered_actions(&mut self) {
        let mut actions = std::mem::take(&mut self.input_actions);
        self.apply_input_actions(&actions);
        actions.clear();
        self.input_actions = actions;
    }

    pub(super) fn handle_focus_lost(&mut self) {
        self.input.clear_focus_state();
        self.ui.pause_from_focus_loss();
    }
}
