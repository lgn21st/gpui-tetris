use gpui_tetris::audio::AudioEngine;
use gpui_tetris::game::input::GameAction;
use gpui_tetris::game::pieces::{Rotation, Tetromino, TetrominoType};
#[cfg(test)]
use gpui_tetris::game::state::GameState;
use std::time::Instant;

use crate::ui::style::DEFAULT_SFX_VOLUME;
use gpui_tetris::game::board::{BOARD_HEIGHT, BOARD_WIDTH};

const BOARD_CELLS: usize = BOARD_WIDTH * BOARD_HEIGHT;

pub struct UiState {
    pub runtime: gpui_tetris::runtime::Runtime,
    pub show_settings: bool,
    pub sfx_volume: f32,
    pub sfx_muted: bool,
    pub audio: Option<AudioEngine>,
    pub(crate) flash_mask: [bool; BOARD_CELLS],
    pub(crate) ghost_mask: [bool; BOARD_CELLS],
    pub(crate) panel_labels: PanelLabels,
    panel_snapshot: Option<PanelSnapshot>,
    pub(crate) preview_cache: PreviewCache,
    pub(crate) board_cache: [Option<TetrominoType>; BOARD_CELLS],
    board_revision: u64,
    active_snapshot: Option<ActiveSnapshot>,
    active_anim: Option<ActiveAnimation>,
}

#[derive(Default)]
pub struct PanelLabels {
    pub score: gpui_kit::SharedString,
    pub level: gpui_kit::SharedString,
    pub lines: gpui_kit::SharedString,
    pub status: gpui_kit::SharedString,
    pub ruleset: gpui_kit::SharedString,
    pub hold: gpui_kit::SharedString,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PanelSnapshot {
    score: u32,
    level: u32,
    lines: u32,
    status: &'static str,
    ruleset: &'static str,
    can_hold: bool,
}

pub const TITLE_HINT: &str = "Press Space or Enter to start";
pub const PAUSED_HINT: &str = "Press P to resume · R to restart";
pub const GAME_OVER_HINT: &str = "Press R to restart";
pub const ONBOARDING_HINTS: [&str; 3] = [
    "Move: Left/Right, Rotate: Up",
    "Soft drop: Down, Hard drop: Space",
    "Hold: C, Pause: P",
];

const PREVIEW_SIZE: usize = 4;

impl UiState {
    #[cfg(test)]
    pub fn new(state: GameState, audio: Option<AudioEngine>) -> Self {
        Self::with_runtime(gpui_tetris::runtime::Runtime::new(state, None), audio)
    }

    pub fn with_runtime(
        runtime: gpui_tetris::runtime::Runtime,
        audio: Option<AudioEngine>,
    ) -> Self {
        let mut ui = Self {
            runtime,
            show_settings: false,
            sfx_volume: DEFAULT_SFX_VOLUME,
            sfx_muted: false,
            audio,
            flash_mask: [false; BOARD_CELLS],
            ghost_mask: [false; BOARD_CELLS],
            panel_labels: PanelLabels::default(),
            panel_snapshot: None,
            preview_cache: PreviewCache::new(),
            board_cache: [None; BOARD_CELLS],
            board_revision: 0,
            active_snapshot: None,
            active_anim: None,
        };
        ui.apply_audio_volume();
        ui.sync_panel_labels();
        ui.active_snapshot = Some(ui.snapshot_active());
        ui
    }

    pub fn receive_action(&mut self, action: GameAction) {
        if self.show_settings {
            return;
        }
        self.runtime.apply_action(action);
    }

    pub fn start_game(&mut self) {
        self.runtime.start();
        self.show_settings = false;
        self.active_snapshot = None;
        self.active_anim = None;
    }

    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.runtime.pause();
        }
    }

    pub fn sync_lifecycle(&mut self) {
        // Remote restart/resume is authoritative; a UI overlay cannot freeze a playable game.
        if self.runtime.playable() {
            self.show_settings = false;
        }
    }

    pub fn close_settings(&mut self) {
        if self.show_settings {
            self.show_settings = false;
        }
    }

    pub fn toggle_mute(&mut self) {
        self.sfx_muted = !self.sfx_muted;
        self.apply_audio_volume();
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.set_volume(self.sfx_volume + delta);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.sfx_muted = false;
        self.sfx_volume = volume.clamp(0.0, 1.0);
        self.apply_audio_volume();
    }

    pub fn reset_settings(&mut self) {
        self.sfx_muted = false;
        self.sfx_volume = DEFAULT_SFX_VOLUME;
        self.apply_audio_volume();
    }

    pub fn apply_audio_volume(&mut self) {
        if let Some(audio) = &self.audio {
            let volume = if self.sfx_muted { 0.0 } else { self.sfx_volume };
            audio.set_master_gain(volume);
        }
    }

    pub fn can_accept_game_input(&self) -> bool {
        self.runtime.started()
            && !self.show_settings
            && !self.runtime.state().paused()
            && !self.runtime.state().game_over()
    }

    pub fn status_label(&self) -> &'static str {
        if !self.runtime.started() {
            "Ready"
        } else if self.runtime.state().game_over() {
            "Game Over"
        } else if self.show_settings {
            "Settings"
        } else if self.runtime.state().paused() {
            "Paused"
        } else {
            "Playing"
        }
    }

    pub fn ruleset_label(&self) -> &'static str {
        if self.runtime.state().is_classic_ruleset() {
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
        self.ghost_mask.fill(false);
    }

    pub fn update_active_animation(&mut self, now: Instant) {
        let current = self.snapshot_active();
        let Some(previous) = self.active_snapshot else {
            self.active_snapshot = Some(current);
            return;
        };

        if previous == current {
            return;
        }

        if previous.kind != current.kind {
            self.active_snapshot = Some(current);
            self.active_anim = None;
            return;
        }

        let dx = current.x - previous.x;
        let dy = current.y - previous.y;
        let rotation_changed = previous.rotation != current.rotation;

        if dy != 0 {
            self.active_snapshot = Some(current);
            self.active_anim = None;
            return;
        }

        if dx.abs() > 1 {
            self.active_snapshot = Some(current);
            self.active_anim = None;
            return;
        }

        let duration_ms = if rotation_changed { 50 } else { 35 };
        self.active_anim = Some(ActiveAnimation {
            from: previous,
            to: current,
            started_at: now,
            duration_ms,
        });
        self.active_snapshot = Some(current);
    }

    pub(crate) fn active_animation_state(&self, now: Instant) -> Option<ActiveAnimationState> {
        let anim = self.active_anim.as_ref()?;
        let elapsed = now.duration_since(anim.started_at).as_millis() as u64;
        if elapsed >= anim.duration_ms {
            return None;
        }

        let progress = (elapsed as f32 / anim.duration_ms as f32).clamp(0.0, 1.0);
        Some(ActiveAnimationState {
            kind: anim.to.kind,
            from_x: anim.from.x,
            from_y: anim.from.y,
            from_rotation: anim.from.rotation,
            to_x: anim.to.x,
            to_y: anim.to.y,
            to_rotation: anim.to.rotation,
            progress,
            rotation_changed: anim.from.rotation != anim.to.rotation,
        })
    }

    pub(crate) fn active_snapshot(&self) -> ActiveSnapshot {
        self.snapshot_active()
    }

    pub fn preview_mask(
        &mut self,
        kind: Option<TetrominoType>,
    ) -> &[[bool; PREVIEW_SIZE]; PREVIEW_SIZE] {
        self.preview_cache.mask(kind)
    }

    pub fn sync_panel_labels(&mut self) {
        let state = self.runtime.state();
        let next = PanelSnapshot {
            score: state.score(),
            level: state.level(),
            lines: state.lines(),
            status: self.status_label(),
            ruleset: self.ruleset_label(),
            can_hold: state.can_hold(),
        };
        let old = self.panel_snapshot;
        if old.is_none_or(|old| old.score != next.score) {
            self.panel_labels.score = format!("Score: {}", next.score).into();
        }
        if old.is_none_or(|old| old.level != next.level) {
            self.panel_labels.level = format!("Level: {}", next.level).into();
        }
        if old.is_none_or(|old| old.lines != next.lines) {
            self.panel_labels.lines = format!("Lines: {}", next.lines).into();
        }
        if old.is_none_or(|old| old.status != next.status) {
            self.panel_labels.status = format!("Status: {}", next.status).into();
        }
        if old.is_none_or(|old| old.ruleset != next.ruleset) {
            self.panel_labels.ruleset = format!("Rules: {}", next.ruleset).into();
        }
        if old.is_none_or(|old| old.can_hold != next.can_hold) {
            self.panel_labels.hold =
                format!("Hold: {}", if next.can_hold { "Ready" } else { "Used" }).into();
        }
        self.panel_snapshot = Some(next);
    }

    pub fn sync_board_cache(&mut self) {
        let revision = self.runtime.state().board_revision();
        if self.board_revision == revision {
            return;
        }
        for (y, row) in self.runtime.state().board().cells.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let idx = y * BOARD_WIDTH + x;
                self.board_cache[idx] = cell.kind;
            }
        }
        self.board_revision = revision;
    }

    fn snapshot_active(&self) -> ActiveSnapshot {
        ActiveSnapshot {
            kind: self.runtime.state().active().kind,
            x: self.runtime.state().active().x,
            y: self.runtime.state().active().y,
            rotation: self.runtime.state().active().rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveSnapshot {
    pub kind: TetrominoType,
    pub x: i32,
    pub y: i32,
    pub rotation: Rotation,
}

#[derive(Clone, Copy, Debug)]
struct ActiveAnimation {
    from: ActiveSnapshot,
    to: ActiveSnapshot,
    started_at: Instant,
    duration_ms: u64,
}

pub(crate) struct ActiveAnimationState {
    pub kind: TetrominoType,
    pub from_x: i32,
    pub from_y: i32,
    pub from_rotation: Rotation,
    pub to_x: i32,
    pub to_y: i32,
    pub to_rotation: Rotation,
    pub progress: f32,
    pub rotation_changed: bool,
}

#[derive(Clone)]
pub struct PreviewCache {
    masks: [Option<[[bool; PREVIEW_SIZE]; PREVIEW_SIZE]>; 7],
}

impl PreviewCache {
    pub fn new() -> Self {
        Self {
            masks: std::array::from_fn(|_| None),
        }
    }

    pub fn mask(&mut self, kind: Option<TetrominoType>) -> &[[bool; PREVIEW_SIZE]; PREVIEW_SIZE] {
        if let Some(kind) = kind {
            let idx = kind as usize;
            return self.masks[idx].get_or_insert_with(|| build_preview_mask(kind));
        }

        static EMPTY: [[bool; PREVIEW_SIZE]; PREVIEW_SIZE] = [[false; PREVIEW_SIZE]; PREVIEW_SIZE];
        &EMPTY
    }
}

fn build_preview_mask(kind: TetrominoType) -> [[bool; PREVIEW_SIZE]; PREVIEW_SIZE] {
    let mut filled = [[false; PREVIEW_SIZE]; PREVIEW_SIZE];
    let piece = Tetromino::new(kind, 0, 0);
    for (x, y) in piece.blocks(piece.rotation).iter() {
        let ux = *x as usize;
        let uy = *y as usize;
        if ux < PREVIEW_SIZE && uy < PREVIEW_SIZE {
            filled[uy][ux] = true;
        }
    }
    filled
}

#[cfg(test)]
mod tests {
    use super::UiState;
    use gpui_tetris::game::input::GameAction;
    use gpui_tetris::game::pieces::TetrominoType;
    #[cfg(test)]
    use gpui_tetris::game::state::GameState;

    #[test]
    fn start_game_sets_started_and_unpauses() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        ui.start_game();

        assert!(ui.runtime.started());
        assert!(!ui.show_settings);
        assert!(!ui.runtime.state().paused());
    }

    #[test]
    fn toggle_settings_pauses_when_opened() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);
        ui.start_game();

        ui.toggle_settings();

        assert!(ui.show_settings);
        assert!(ui.runtime.state().paused());
    }

    #[test]
    fn volume_label_reflects_muted_state() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        assert_eq!(ui.sfx_volume_label(), "70%");
        ui.toggle_mute();
        ui.sync_panel_labels();
        assert_eq!(ui.sfx_volume_label(), "Muted");
    }

    #[test]
    fn setting_volume_from_control_clamps_and_unmutes() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);
        ui.toggle_mute();

        ui.set_volume(1.5);

        assert_eq!(ui.sfx_volume, 1.0);
        assert!(!ui.sfx_muted);
        assert_eq!(ui.sfx_volume_label(), "100%");
    }

    #[test]
    fn remote_restart_closes_settings_and_restores_authoritative_playability() {
        let mut ui = UiState::new(GameState::new(1, Default::default()), None);
        ui.start_game();
        ui.toggle_settings();
        assert!(ui.runtime.state().paused());
        ui.runtime.apply_action(GameAction::Restart);
        ui.sync_lifecycle();
        assert!(!ui.show_settings);
        assert!(ui.can_accept_game_input());
    }

    #[test]
    fn settings_block_game_actions_from_component_interactions() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);
        ui.start_game();
        ui.toggle_settings();
        let active_before = ui.active_snapshot();

        ui.receive_action(GameAction::MoveLeft);

        assert_eq!(ui.active_snapshot(), active_before);
        assert!(ui.runtime.state().paused());
    }

    #[test]
    fn preview_mask_has_blocks_for_piece() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);

        let mask = ui.preview_mask(Some(TetrominoType::I));
        let any_filled = mask.iter().flatten().any(|filled| *filled);
        assert!(any_filled);

        let empty = ui.preview_mask(None);
        let any_empty_filled = empty.iter().flatten().any(|filled| *filled);
        assert!(!any_empty_filled);
    }

    #[test]
    fn title_input_starts_game_for_mapped_actions() {
        let state = GameState::new(1, Default::default());
        let mut ui = UiState::new(state, None);
        assert!(!ui.runtime.started());

        ui.receive_action(GameAction::MoveLeft);

        assert!(ui.runtime.started());
    }
}
