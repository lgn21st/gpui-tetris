use crate::game::input::GameAction;
use crate::game::pieces::{Tetromino, TetrominoType, spawn_position};

use super::kicks::srs_kicks;
use super::scoring::t_spin_kind;
use super::{GameState, Ruleset, SoundEvent, TSpinKind};

pub(super) fn apply_action(state: &mut GameState, action: GameAction) {
    if state.game_over && action != GameAction::Restart {
        return;
    }
    if state.paused && action != GameAction::Pause && action != GameAction::Restart {
        return;
    }

    match action {
        GameAction::MoveLeft => {
            try_move(state, -1, 0);
            state.last_action_rotate = false;
            state.sound_events.push(SoundEvent::Move);
        }
        GameAction::MoveRight => {
            try_move(state, 1, 0);
            state.last_action_rotate = false;
            state.sound_events.push(SoundEvent::Move);
        }
        GameAction::SoftDrop => {
            if try_move(state, 0, 1) {
                state.score = state.score.saturating_add(1);
            }
            activate_soft_drop(state);
            state.last_action_rotate = false;
            state.sound_events.push(SoundEvent::SoftDrop);
        }
        GameAction::HardDrop => {
            let mut dropped = 0;
            while try_move(state, 0, 1) {
                dropped += 1;
            }
            if dropped > 0 {
                state.score = state.score.saturating_add(dropped * 2);
            }
            state.sound_events.push(SoundEvent::HardDrop);
            lock_active_piece(state);
            state.lock_timer_ms = 0;
            state.drop_timer_ms = 0;
        }
        GameAction::RotateCw => {
            state.last_action_rotate = try_rotate(state, true);
            state.sound_events.push(SoundEvent::Rotate);
        }
        GameAction::RotateCcw => {
            state.last_action_rotate = try_rotate(state, false);
            state.sound_events.push(SoundEvent::Rotate);
        }
        GameAction::Hold => {
            if !state.can_hold {
                return;
            }

            let current_kind = state.active.kind;
            if let Some(held_kind) = state.hold {
                state.hold = Some(current_kind);
                state.active = spawn_piece(state, held_kind);
            } else {
                state.hold = Some(current_kind);
                state.spawn_next();
            }
            state.can_hold = false;
            state.last_action_rotate = false;
            state.sound_events.push(SoundEvent::Hold);
        }
        GameAction::Pause => {
            state.paused = !state.paused;
        }
        GameAction::Restart => {
            state.reset();
        }
    }
}

pub(super) fn spawn_piece(state: &mut GameState, kind: TetrominoType) -> Tetromino {
    let (spawn_x, spawn_y) = spawn_position();
    let piece = Tetromino::new(kind, spawn_x, spawn_y);
    if !state
        .board
        .can_place(&piece, piece.x, piece.y, piece.rotation)
    {
        state.game_over = true;
        state.sound_events.push(SoundEvent::GameOver);
    }
    piece
}

pub(super) fn activate_soft_drop(state: &mut GameState) {
    state.soft_drop_active = true;
    state.soft_drop_timeout_ms = state.soft_drop_grace_ms;
}

pub(super) fn ghost_blocks(state: &GameState) -> [(i32, i32); 4] {
    let mut ghost_y = state.active.y;
    while state.board.can_place(
        &state.active,
        state.active.x,
        ghost_y + 1,
        state.active.rotation,
    ) {
        ghost_y += 1;
    }

    let mut blocks = state.active.blocks(state.active.rotation);
    for (x, y) in blocks.iter_mut() {
        *x += state.active.x;
        *y += ghost_y;
    }
    blocks
}

pub(super) fn try_move(state: &mut GameState, dx: i32, dy: i32) -> bool {
    let new_x = state.active.x + dx;
    let new_y = state.active.y + dy;
    if state
        .board
        .can_place(&state.active, new_x, new_y, state.active.rotation)
    {
        state.active.x = new_x;
        state.active.y = new_y;
        handle_lock_reset(state);
        return true;
    }
    false
}

pub(super) fn try_rotate(state: &mut GameState, clockwise: bool) -> bool {
    let next_rotation = if clockwise {
        state.active.rotation.cw()
    } else {
        state.active.rotation.ccw()
    };
    let kicks = srs_kicks(state.active.kind, state.active.rotation, next_rotation);
    for (dx, dy) in kicks.iter() {
        let new_x = state.active.x + dx;
        let new_y = state.active.y + dy;
        if state
            .board
            .can_place(&state.active, new_x, new_y, next_rotation)
        {
            state.active.x = new_x;
            state.active.y = new_y;
            state.active.rotation = next_rotation;
            handle_lock_reset(state);
            return true;
        }
    }

    false
}

pub(super) fn can_move_down(state: &GameState) -> bool {
    state.board.can_place(
        &state.active,
        state.active.x,
        state.active.y + 1,
        state.active.rotation,
    )
}

pub(super) fn handle_lock_reset(state: &mut GameState) {
    if can_move_down(state) {
        state.lock_timer_ms = 0;
        state.lock_reset_count = 0;
        return;
    }

    if state.lock_reset_count < state.lock_reset_limit {
        state.lock_timer_ms = 0;
        state.lock_reset_count += 1;
    }
}

pub(super) fn lock_active_piece(state: &mut GameState) {
    let t_spin = if state.ruleset == Ruleset::Modern {
        t_spin_kind(state)
    } else {
        TSpinKind::None
    };
    set_landing_flash(state);
    state.board.lock_piece(&state.active);
    let cleared = state.board.clear_lines();
    state.apply_line_clear(cleared, t_spin);
    state.spawn_next();
    state.last_action_rotate = false;
}

pub(super) fn set_landing_flash(state: &mut GameState) {
    let blocks = state.active.blocks(state.active.rotation);
    for (index, (dx, dy)) in blocks.iter().enumerate() {
        state.last_lock_cells[index] = (state.active.x + dx, state.active.y + dy);
    }
    state.landing_flash_timer_ms = 120;
}
