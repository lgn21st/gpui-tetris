use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use gpui_tetris::game::input::{GameAction, RepeatConfig, RepeatState};

use crate::ui::style::CONTROLLER_AXIS_THRESHOLD;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisDirection {
    Left,
    Right,
}

pub struct InputState {
    repeat_config: RepeatConfig,
    soft_drop_repeat_config: RepeatConfig,
    left_repeat: RepeatState,
    right_repeat: RepeatState,
    down_repeat: RepeatState,
    keyboard_left_held: bool,
    keyboard_right_held: bool,
    last_dir: Option<AxisDirection>,
    gilrs: Option<Gilrs>,
    gamepad_id: Option<GamepadId>,
    controller_left_button: bool,
    controller_right_button: bool,
    controller_down_button: bool,
    controller_left_axis: bool,
    controller_right_axis: bool,
    controller_down_axis: bool,
    controller_left_held: bool,
    controller_right_held: bool,
    controller_down_held: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct InputAction {
    pub action: GameAction,
    pub record: bool,
}

impl InputAction {
    fn recorded(action: GameAction) -> Self {
        Self {
            action,
            record: true,
        }
    }

    fn silent(action: GameAction) -> Self {
        Self {
            action,
            record: false,
        }
    }
}

impl InputState {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().ok();
        let gamepad_id = gilrs
            .as_ref()
            .and_then(|gilrs| gilrs.gamepads().next().map(|(id, _)| id));
        Self {
            repeat_config: RepeatConfig::default(),
            soft_drop_repeat_config: RepeatConfig {
                das_ms: 0,
                arr_ms: 50,
            },
            left_repeat: RepeatState::new(),
            right_repeat: RepeatState::new(),
            down_repeat: RepeatState::new(),
            keyboard_left_held: false,
            keyboard_right_held: false,
            last_dir: None,
            gilrs,
            gamepad_id,
            controller_left_button: false,
            controller_right_button: false,
            controller_down_button: false,
            controller_left_axis: false,
            controller_right_axis: false,
            controller_down_axis: false,
            controller_left_held: false,
            controller_right_held: false,
            controller_down_held: false,
        }
    }

    pub fn set_keyboard_left(&mut self, held: bool) -> Vec<InputAction> {
        self.keyboard_left_held = held;
        self.sync_movement_holds()
    }

    pub fn set_keyboard_right(&mut self, held: bool) -> Vec<InputAction> {
        self.keyboard_right_held = held;
        self.sync_movement_holds()
    }

    pub fn clear_focus_state(&mut self) {
        self.keyboard_left_held = false;
        self.keyboard_right_held = false;
        self.clear_controller_state();
    }

    pub fn poll_controller(&mut self) -> Vec<InputAction> {
        let Some(mut gilrs) = self.gilrs.take() else {
            return Vec::new();
        };
        let mut actions = Vec::with_capacity(8);
        while let Some(event) = gilrs.next_event() {
            if self.gamepad_id.is_none() {
                self.gamepad_id = Some(event.id);
            }

            if let Some(active) = self.gamepad_id {
                if event.id != active {
                    continue;
                }
            } else {
                continue;
            }

            match event.event {
                EventType::Connected => {}
                EventType::Disconnected => {
                    if self.gamepad_id == Some(event.id) {
                        self.clear_controller_state();
                        self.gamepad_id = None;
                    }
                }
                EventType::ButtonPressed(button, _) => {
                    actions.extend(self.handle_controller_button(button, true));
                }
                EventType::ButtonReleased(button, _) => {
                    actions.extend(self.handle_controller_button(button, false));
                }
                EventType::AxisChanged(axis, value, _) => {
                    actions.extend(self.handle_controller_axis(axis, value));
                }
                _ => {}
            }
        }
        self.gilrs = Some(gilrs);

        actions
    }

    pub fn apply_repeats(&mut self, elapsed_ms: u64, can_accept: bool) -> Vec<InputAction> {
        if !can_accept {
            self.left_repeat.release();
            self.right_repeat.release();
            self.down_repeat.release();
            self.last_dir = None;
            return Vec::new();
        }
        let direction = match (self.left_repeat.is_held(), self.right_repeat.is_held()) {
            (true, false) => Some(AxisDirection::Left),
            (false, true) => Some(AxisDirection::Right),
            (true, true) => self.last_dir,
            _ => None,
        };

        let mut actions = Vec::new();
        match direction {
            Some(AxisDirection::Left) => {
                let count = self.left_repeat.tick(elapsed_ms, &self.repeat_config);
                for _ in 0..count {
                    actions.push(InputAction::recorded(GameAction::MoveLeft));
                }
            }
            Some(AxisDirection::Right) => {
                let count = self.right_repeat.tick(elapsed_ms, &self.repeat_config);
                for _ in 0..count {
                    actions.push(InputAction::recorded(GameAction::MoveRight));
                }
            }
            None => {}
        }

        if self.down_repeat.is_held() {
            let count = self
                .down_repeat
                .tick(elapsed_ms, &self.soft_drop_repeat_config);
            for _ in 0..count {
                actions.push(InputAction::recorded(GameAction::SoftDrop));
            }
        }

        actions
    }

    fn handle_controller_button(&mut self, button: Button, pressed: bool) -> Vec<InputAction> {
        let mut actions = Vec::new();
        match button {
            Button::DPadLeft => self.controller_left_button = pressed,
            Button::DPadRight => self.controller_right_button = pressed,
            Button::DPadDown => self.controller_down_button = pressed,
            Button::South if pressed => actions.push(InputAction::silent(GameAction::RotateCw)),
            Button::East if pressed => actions.push(InputAction::silent(GameAction::RotateCcw)),
            Button::West if pressed => actions.push(InputAction::silent(GameAction::Hold)),
            Button::North if pressed => actions.push(InputAction::silent(GameAction::HardDrop)),
            Button::Start if pressed => actions.push(InputAction::silent(GameAction::Pause)),
            Button::Select | Button::Mode if pressed => {
                actions.push(InputAction::silent(GameAction::Restart))
            }
            _ => {}
        }

        actions.extend(self.sync_controller_holds());
        actions
    }

    fn handle_controller_axis(&mut self, axis: Axis, value: f32) -> Vec<InputAction> {
        match axis {
            Axis::LeftStickX => {
                self.controller_left_axis = value < -CONTROLLER_AXIS_THRESHOLD;
                self.controller_right_axis = value > CONTROLLER_AXIS_THRESHOLD;
            }
            Axis::LeftStickY => {
                self.controller_down_axis = value > CONTROLLER_AXIS_THRESHOLD;
            }
            _ => {}
        }

        self.sync_controller_holds()
    }

    fn sync_movement_holds(&mut self) -> Vec<InputAction> {
        let left = self.keyboard_left_held || self.controller_left_held;
        let right = self.keyboard_right_held || self.controller_right_held;
        let mut actions = Vec::new();

        if left != self.left_repeat.is_held() {
            if left {
                if self.left_repeat.press() {
                    actions.push(InputAction::recorded(GameAction::MoveLeft));
                }
            } else {
                self.left_repeat.release();
            }
        }

        if right != self.right_repeat.is_held() {
            if right {
                if self.right_repeat.press() {
                    actions.push(InputAction::recorded(GameAction::MoveRight));
                }
            } else {
                self.right_repeat.release();
            }
        }

        match (left, right) {
            (true, false) => self.last_dir = Some(AxisDirection::Left),
            (false, true) => self.last_dir = Some(AxisDirection::Right),
            (false, false) => self.last_dir = None,
            (true, true) => {}
        }

        actions
    }

    fn sync_controller_holds(&mut self) -> Vec<InputAction> {
        let left = self.controller_left_button || self.controller_left_axis;
        let right = self.controller_right_button || self.controller_right_axis;
        let down = self.controller_down_button || self.controller_down_axis;

        if left != self.controller_left_held {
            self.controller_left_held = left;
        }

        if right != self.controller_right_held {
            self.controller_right_held = right;
        }

        let mut actions = self.sync_movement_holds();

        if down != self.controller_down_held {
            self.controller_down_held = down;
            if down {
                if self.down_repeat.press() {
                    actions.push(InputAction::recorded(GameAction::SoftDrop));
                }
            } else {
                self.down_repeat.release();
            }
        }

        actions
    }

    fn clear_controller_state(&mut self) {
        self.controller_left_button = false;
        self.controller_right_button = false;
        self.controller_down_button = false;
        self.controller_left_axis = false;
        self.controller_right_axis = false;
        self.controller_down_axis = false;
        self.controller_left_held = false;
        self.controller_right_held = false;
        self.controller_down_held = false;
        self.down_repeat.release();
        let _ = self.sync_movement_holds();
    }
}

#[cfg(test)]
mod tests {
    use super::InputState;
    use gpui_tetris::game::input::GameAction;

    #[test]
    fn keyboard_press_emits_single_move() {
        let mut input = InputState::new();
        let actions = input.set_keyboard_left(true);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, GameAction::MoveLeft);
        assert!(actions[0].record);
    }

    #[test]
    fn repeat_emits_after_das_and_arr() {
        let mut input = InputState::new();
        let _ = input.set_keyboard_left(true);

        let actions = input.apply_repeats(150, true);
        assert!(actions.is_empty());

        let actions = input.apply_repeats(50, true);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, GameAction::MoveLeft);
    }
}
