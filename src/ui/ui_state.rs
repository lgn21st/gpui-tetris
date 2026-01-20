use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::state::GameState;

use crate::ui::style::{BOARD_COLS_USIZE, BOARD_ROWS_USIZE, DEFAULT_SFX_VOLUME};

const BOARD_CELLS: usize = BOARD_COLS_USIZE * BOARD_ROWS_USIZE;

pub struct UiState {
    pub last_action: Option<GameAction>,
    pub state: GameState,
    pub started: bool,
    pub show_settings: bool,
    pub sfx_volume: f32,
    pub sfx_muted: bool,
    pub audio: Option<AudioEngine>,
    pub(crate) flash_mask: [bool; BOARD_CELLS],
    pub(crate) active_mask: [bool; BOARD_CELLS],
    pub(crate) ghost_mask: [bool; BOARD_CELLS],
    pub(crate) panel_labels: PanelLabels,
    labels_dirty: bool,
}

#[derive(Default)]
pub struct PanelLabels {
    pub last_input: String,
    pub score: String,
    pub level: String,
    pub lines: String,
    pub status: String,
    pub ruleset: String,
    pub hold: String,
    pub grounded: String,
    pub lock_resets: String,
    pub sfx: String,
    pub combo: String,
    pub b2b: String,
}

pub const SETTINGS_SHORTCUTS: &str = "M: mute · +/-: volume · 0: reset";
pub const SETTINGS_BACK: &str = "S or Esc: back";
pub const TITLE_HINT: &str = "Press Enter or Space to Start";
pub const TITLE_SETTINGS: &str = "S: Settings";
pub const FOCUS_HINT: &str = "Click to Focus";
pub const PAUSED_HINT: &str = "Press P to resume";
pub const GAME_OVER_HINT: &str = "Press R to restart";

impl UiState {
    pub fn new(state: GameState, audio: Option<AudioEngine>) -> Self {
        let mut ui = Self {
            last_action: None,
            state,
            started: false,
            show_settings: false,
            sfx_volume: DEFAULT_SFX_VOLUME,
            sfx_muted: false,
            audio,
            flash_mask: [false; BOARD_CELLS],
            active_mask: [false; BOARD_CELLS],
            ghost_mask: [false; BOARD_CELLS],
            panel_labels: PanelLabels::default(),
            labels_dirty: true,
        };
        ui.apply_audio_volume();
        ui.sync_panel_labels();
        ui
    }

    pub fn receive_action(&mut self, action: GameAction) {
        self.apply_action(action, true);
    }

    pub fn apply_action(&mut self, action: GameAction, record: bool) {
        if record {
            self.last_action = Some(action);
        }
        if !self.started {
            if matches!(action, GameAction::Restart | GameAction::HardDrop) {
                self.start_game();
            }
            self.mark_labels_dirty();
            return;
        }
        if self.show_settings {
            self.mark_labels_dirty();
            return;
        }

        self.state.apply_action(action);
        if action == GameAction::Restart {
            self.started = true;
        }
        self.mark_labels_dirty();
    }

    pub fn start_game(&mut self) {
        self.started = true;
        self.show_settings = false;
        self.state.reset();
        self.state.paused = false;
        self.mark_labels_dirty();
    }

    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings && !self.state.game_over {
            self.state.paused = true;
        }
        self.mark_labels_dirty();
    }

    pub fn close_settings(&mut self) {
        if self.show_settings {
            self.show_settings = false;
            self.mark_labels_dirty();
        }
    }

    pub fn toggle_mute(&mut self) {
        self.sfx_muted = !self.sfx_muted;
        self.apply_audio_volume();
        self.mark_labels_dirty();
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        if self.sfx_muted {
            self.sfx_muted = false;
        }
        self.sfx_volume = (self.sfx_volume + delta).clamp(0.0, 1.0);
        self.apply_audio_volume();
        self.mark_labels_dirty();
    }

    pub fn reset_settings(&mut self) {
        self.sfx_muted = false;
        self.sfx_volume = DEFAULT_SFX_VOLUME;
        self.apply_audio_volume();
        self.mark_labels_dirty();
    }

    pub fn apply_audio_volume(&mut self) {
        if let Some(audio) = &self.audio {
            let volume = if self.sfx_muted { 0.0 } else { self.sfx_volume };
            audio.set_master_gain(volume);
        }
    }

    pub fn can_accept_game_input(&self) -> bool {
        self.started && !self.show_settings && !self.state.paused && !self.state.game_over
    }

    pub fn status_label(&self) -> &'static str {
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

    pub fn ruleset_label(&self) -> &'static str {
        if self.state.is_classic_ruleset() {
            "Classic"
        } else {
            "Modern"
        }
    }

    pub fn sfx_volume_label(&self) -> String {
        if self.sfx_muted {
            "Muted".to_string()
        } else {
            format!("{:.0}%", self.sfx_volume * 100.0)
        }
    }

    pub fn clear_render_masks(&mut self) {
        self.flash_mask.fill(false);
        self.active_mask.fill(false);
        self.ghost_mask.fill(false);
    }

    pub fn sync_panel_labels(&mut self) {
        if !self.labels_dirty {
            return;
        }
        self.update_panel_labels();
        self.labels_dirty = false;
    }

    pub fn mark_labels_dirty(&mut self) {
        self.labels_dirty = true;
    }

    pub fn pause_from_focus_loss(&mut self) {
        if self.started && !self.state.game_over {
            self.state.paused = true;
            self.mark_labels_dirty();
        }
    }

    fn update_panel_labels(&mut self) {
        let last_input = self
            .last_action
            .as_ref()
            .map(action_label)
            .unwrap_or("None");
        self.panel_labels.last_input = format!("Last input: {}", last_input);
        self.panel_labels.score = format!("Score: {}", self.state.score);
        self.panel_labels.level = format!("Level: {}", self.state.level);
        self.panel_labels.lines = format!("Lines: {}", self.state.lines);
        self.panel_labels.status = format!("Status: {}", self.status_label());
        self.panel_labels.ruleset = format!("Rules: {}", self.ruleset_label());
        self.panel_labels.hold = format!(
            "Hold: {}",
            if self.state.can_hold { "Ready" } else { "Used" }
        );
        self.panel_labels.grounded = format!(
            "Grounded: {}",
            if self.state.is_grounded() {
                "Yes"
            } else {
                "No"
            }
        );
        self.panel_labels.lock_resets = format!(
            "Lock resets: {}/{}",
            self.state.lock_reset_remaining(),
            self.state.lock_reset_limit
        );
        self.panel_labels.sfx = format!("SFX: {}", self.sfx_volume_label());
        self.panel_labels.combo = format!(
            "Combo: {}",
            if self.state.combo >= 0 {
                self.state.combo.to_string()
            } else {
                "-".to_string()
            }
        );
        self.panel_labels.b2b = format!(
            "B2B: {}",
            if self.state.back_to_back { "Yes" } else { "No" }
        );
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

#[cfg(test)]
mod tests {
    use super::UiState;
    use gpui_tetris::game::input::GameAction;
    use gpui_tetris::game::state::GameState;

    #[test]
    fn start_game_sets_started_and_unpauses() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        ui.start_game();

        assert!(ui.started);
        assert!(!ui.show_settings);
        assert!(!ui.state.paused);
    }

    #[test]
    fn toggle_settings_pauses_when_opened() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);
        ui.started = true;

        ui.toggle_settings();

        assert!(ui.show_settings);
        assert!(ui.state.paused);
    }

    #[test]
    fn volume_label_reflects_muted_state() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        assert_eq!(ui.sfx_volume_label(), "70%");
        ui.toggle_mute();
        assert_eq!(ui.sfx_volume_label(), "Muted");
    }

    #[test]
    fn receive_action_records_last_action() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        ui.receive_action(GameAction::Pause);

        assert_eq!(ui.last_action, Some(GameAction::Pause));
    }
}
