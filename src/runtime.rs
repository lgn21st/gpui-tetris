//! Window-independent authoritative loop. Rendering is a reader, never its clock.
use crate::adapter::SocketAdapter;
use crate::game::{
    input::GameAction,
    state::{GameState, SoundEvent},
};
use std::time::{Duration, Instant};

const MAX_CATCH_UP_STEPS: u32 = 15;

pub struct Runtime {
    state: GameState,
    adapter: Option<SocketAdapter>,
    started: bool,
    clock: FixedClock,
    actions: Vec<GameAction>,
}

impl Runtime {
    pub fn new(state: GameState, adapter: Option<SocketAdapter>) -> Self {
        let autostart = adapter.is_some();
        let mut runtime = Self {
            state,
            adapter,
            started: false,
            clock: FixedClock::default(),
            actions: Vec::with_capacity(32),
        };
        if autostart {
            runtime.start();
        }
        runtime
    }
    pub fn state(&self) -> &GameState {
        &self.state
    }
    pub fn started(&self) -> bool {
        self.started
    }
    pub fn playable(&self) -> bool {
        self.started && !self.state.paused() && !self.state.game_over()
    }

    pub fn start(&mut self) {
        self.started = true;
        self.state.apply_action(GameAction::Restart);
        self.clock.remainder = Duration::ZERO;
    }

    pub fn apply_action(&mut self, action: GameAction) {
        if !self.started {
            self.start();
            if matches!(
                action,
                GameAction::Pause | GameAction::Restart | GameAction::HardDrop
            ) {
                return;
            }
        }
        self.state.apply_action(action);
        if matches!(action, GameAction::Pause | GameAction::Restart) {
            self.clock.remainder = Duration::ZERO;
        }
    }

    pub fn pause(&mut self) {
        if !self.state.paused() && !self.state.game_over() {
            self.state.apply_action(GameAction::Pause);
            self.clock.remainder = Duration::ZERO;
        }
    }

    /// One bounded pump: accepted commands, fixed steps with local repeats, then snapshot.
    /// The repeat source supplies actions only, never mutable access to the game.
    pub fn pump(&mut self, now: Instant, mut repeats: impl FnMut(u64, bool, &mut Vec<GameAction>)) {
        let previous_lifecycle = (self.state.episode_id(), self.state.paused());
        if let Some(adapter) = &mut self.adapter {
            adapter.poll_and_apply_at(&mut self.state, now);
        }
        if previous_lifecycle != (self.state.episode_id(), self.state.paused()) {
            self.clock.remainder = Duration::ZERO;
        }
        let step = Duration::from_millis(self.state.tick_ms().max(1));
        let steps = self.clock.advance(now, step, self.playable());
        for _ in 0..steps {
            self.state.tick(step.as_millis() as u64, false);
            self.actions.clear();
            repeats(step.as_millis() as u64, self.playable(), &mut self.actions);
            for action in self.actions.drain(..).take(32) {
                self.state.apply_action(action);
            }
        }
        if let Some(adapter) = &mut self.adapter {
            adapter.emit_observation(&mut self.state);
        }
    }

    pub fn drain_sounds(&mut self) -> impl Iterator<Item = SoundEvent> + '_ {
        self.state.drain_sound_events()
    }
}

#[derive(Default)]
struct FixedClock {
    last: Option<Instant>,
    remainder: Duration,
}
impl FixedClock {
    fn advance(&mut self, now: Instant, step: Duration, running: bool) -> u32 {
        let elapsed = self
            .last
            .replace(now)
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        if !running {
            self.remainder = Duration::ZERO;
            return 0;
        }
        self.remainder += elapsed.min(step.saturating_mul(MAX_CATCH_UP_STEPS));
        let mut steps = 0;
        while self.remainder >= step && steps < MAX_CATCH_UP_STEPS {
            self.remainder -= step;
            steps += 1;
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_wall_time_has_equal_steps_across_refresh_rates() {
        for hz in [60, 120, 144, 1000, 2000] {
            let mut clock = FixedClock::default();
            let start = Instant::now();
            let step = Duration::from_millis(16);
            clock.advance(start, step, true);
            let mut count = 0;
            for frame in 1..=hz * 60 {
                let elapsed = Duration::from_nanos(frame * 60_000_000_000 / (hz * 60));
                count += clock.advance(start + elapsed, step, true);
            }
            assert_eq!(count, 3750, "{hz} Hz");
        }
    }
    #[test]
    fn stalled_clock_is_bounded_and_pause_discards_elapsed() {
        let mut clock = FixedClock::default();
        let now = Instant::now();
        let step = Duration::from_millis(16);
        clock.advance(now, step, true);
        assert_eq!(
            clock.advance(now + Duration::from_secs(60), step, true),
            MAX_CATCH_UP_STEPS
        );
        assert_eq!(
            clock.advance(now + Duration::from_secs(120), step, false),
            0
        );
        assert_eq!(
            clock.advance(now + Duration::from_secs(120) + step, step, true),
            1
        );
    }
    #[test]
    fn runtime_advances_without_a_window_and_pause_is_authoritative() {
        let mut runtime = Runtime::new(GameState::new(1, Default::default()), None);
        runtime.start();
        let now = Instant::now();
        runtime.pump(now, |_, _, _| {});
        let before = runtime.state().logical_step();
        runtime.pump(now + Duration::from_millis(32), |_, _, _| {});
        assert_eq!(runtime.state().logical_step(), before + 2);
        runtime.pause();
        let paused = runtime.state().logical_step();
        runtime.pump(now + Duration::from_secs(1), |_, _, _| {});
        assert_eq!(runtime.state().logical_step(), paused);
        runtime.apply_action(GameAction::Restart);
        assert!(runtime.playable());
    }
}
