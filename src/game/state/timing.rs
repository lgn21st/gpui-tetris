use super::GameState;

const DROP_INTERVALS_MS: [u64; 9] = [1000, 800, 650, 500, 400, 320, 250, 200, 160];

pub(super) fn drop_interval_ms(state: &GameState, soft_drop: bool) -> u64 {
    let mut interval = DROP_INTERVALS_MS
        .get(state.level as usize)
        .copied()
        .unwrap_or(120);
    interval = interval.min(state.base_drop_ms).max(100);
    if soft_drop {
        let adjusted = interval / state.soft_drop_multiplier.max(1);
        return adjusted.max(1);
    }
    interval
}

pub(super) fn tick(state: &mut GameState, elapsed_ms: u64, soft_drop: bool) {
    if state.game_over || state.paused {
        return;
    }

    step_landing_flash(state, elapsed_ms);
    if step_line_clear_pause(state, elapsed_ms) {
        return;
    }
    update_drop_timers(state, elapsed_ms);
    apply_gravity_steps(state, soft_drop);
    update_lock_timer(state, elapsed_ms);
}

fn step_landing_flash(state: &mut GameState, elapsed_ms: u64) {
    if state.landing_flash_timer_ms > 0 {
        state.landing_flash_timer_ms = state.landing_flash_timer_ms.saturating_sub(elapsed_ms);
    }
}

fn step_line_clear_pause(state: &mut GameState, elapsed_ms: u64) -> bool {
    if state.line_clear_timer_ms > 0 {
        state.line_clear_timer_ms = state.line_clear_timer_ms.saturating_sub(elapsed_ms);
        return state.line_clear_timer_ms > 0;
    }
    false
}

fn update_drop_timers(state: &mut GameState, elapsed_ms: u64) {
    state.drop_timer_ms = state.drop_timer_ms.saturating_add(elapsed_ms);
    if state.soft_drop_timeout_ms > 0 {
        state.soft_drop_timeout_ms = state.soft_drop_timeout_ms.saturating_sub(elapsed_ms);
        if state.soft_drop_timeout_ms == 0 {
            state.soft_drop_active = false;
        }
    }
}

fn apply_gravity_steps(state: &mut GameState, soft_drop: bool) {
    let interval = drop_interval_ms(state, soft_drop || state.soft_drop_active);
    while state.drop_timer_ms >= interval {
        state.drop_timer_ms -= interval;
        let _ = state.try_move(0, 1);
    }
}

fn update_lock_timer(state: &mut GameState, elapsed_ms: u64) {
    if state.can_move_down() {
        state.lock_timer_ms = 0;
        state.lock_reset_count = 0;
    } else {
        state.lock_timer_ms = state.lock_timer_ms.saturating_add(elapsed_ms);
        if state.lock_timer_ms >= state.lock_delay_ms {
            state.lock_timer_ms = 0;
            state.drop_timer_ms = 0;
            state.lock_active_piece();
        }
    }
}
