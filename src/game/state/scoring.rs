use crate::game::pieces::{Rotation, TetrominoType};

use super::{GameState, Ruleset, SoundEvent, TSpinKind};

pub(super) fn apply_line_clear(state: &mut GameState, cleared: usize, t_spin: TSpinKind) {
    let qualifies_b2b = (t_spin == TSpinKind::Full && cleared > 0) || cleared == 4;
    let level = state.level + 1;
    let mut points = if state.ruleset == Ruleset::Classic {
        match cleared {
            1 => state.rules.classic_line_scores[0],
            2 => state.rules.classic_line_scores[1],
            3 => state.rules.classic_line_scores[2],
            4 => state.rules.classic_line_scores[3],
            _ => 0,
        }
    } else {
        match t_spin {
            TSpinKind::Full => match cleared {
                0 => state.rules.t_spin_full[0],
                1 => state.rules.t_spin_full[1],
                2 => state.rules.t_spin_full[2],
                3 => state.rules.t_spin_full[3],
                _ => 0,
            },
            TSpinKind::Mini => match cleared {
                0 => state.rules.t_spin_mini[0],
                1 => state.rules.t_spin_mini[1],
                2 => state.rules.t_spin_mini[2],
                _ => 0,
            },
            TSpinKind::None => match cleared {
                1 => state.rules.classic_line_scores[0],
                2 => state.rules.classic_line_scores[1],
                3 => state.rules.classic_line_scores[2],
                4 => state.rules.classic_line_scores[3],
                _ => 0,
            },
        }
    };

    if state.ruleset == Ruleset::Modern && qualifies_b2b && state.back_to_back {
        points = points * state.rules.b2b_bonus_num / state.rules.b2b_bonus_den;
    }

    if cleared > 0 {
        state.line_clear_timer_ms = 180;
        state
            .sound_events
            .push(SoundEvent::LineClear(cleared as u8));
        state.lines += cleared as u32;
        if state.ruleset == Ruleset::Modern {
            state.combo += 1;
            if state.combo > 0 {
                points += state.rules.combo_base * state.combo as u32;
            }
            state.back_to_back = qualifies_b2b;
        } else {
            state.combo = -1;
            state.back_to_back = false;
        }

        // Classic progression: advance level every 10 lines.
        state.level = state.lines / 10;
    } else {
        state.combo = -1;
        state.back_to_back = false;
    }

    if points > 0 {
        state.score += points * level;
    }
}

pub(super) fn t_spin_kind(state: &GameState) -> TSpinKind {
    if state.active.kind != TetrominoType::T || !state.last_action_rotate {
        return TSpinKind::None;
    }

    let center_x = state.active.x + 1;
    let center_y = state.active.y + 1;
    let corners = [
        (center_x - 1, center_y - 1),
        (center_x + 1, center_y - 1),
        (center_x - 1, center_y + 1),
        (center_x + 1, center_y + 1),
    ];
    let mut filled = 0;
    for (x, y) in corners.iter() {
        if state.board.is_occupied(*x, *y) {
            filled += 1;
        }
    }
    if filled < 3 {
        return TSpinKind::None;
    }

    let (front_a, front_b) = match state.active.rotation {
        Rotation::North => ((center_x - 1, center_y - 1), (center_x + 1, center_y - 1)),
        Rotation::East => ((center_x + 1, center_y - 1), (center_x + 1, center_y + 1)),
        Rotation::South => ((center_x - 1, center_y + 1), (center_x + 1, center_y + 1)),
        Rotation::West => ((center_x - 1, center_y - 1), (center_x - 1, center_y + 1)),
    };
    let front_filled = state.board.is_occupied(front_a.0, front_a.1) as u8
        + state.board.is_occupied(front_b.0, front_b.1) as u8;

    if front_filled == 2 {
        TSpinKind::Full
    } else {
        TSpinKind::Mini
    }
}
