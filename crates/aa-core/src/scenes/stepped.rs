//! Fixed-timestep driver shared by the stateful scenes (Matrix, Life,
//! pipes).
//!
//! Port of `SteppedScene.swift`. The Swift original is an open base *class* the
//! concrete scenes subclass; Rust has no inheritance, so the shared machinery
//! lives here as a plain struct ([`Stepper`]) that each scene *holds* and drives.
//! It turns the host's "give me the frame at time `t`" pull into a fixed-timestep
//! simulation: it accumulates wall-clock `dt` and emits the number of `step()`s
//! that should run before rendering, independent of the display refresh rate.
//!
//! Usage from a scene's `frame(&mut self, t)`:
//! ```ignore
//! let steps = self.stepper.advance(t, self.step_interval());
//! for _ in 0..steps { self.step(); }
//! self.render()
//! ```

/// Accumulates wall-clock time and converts it into discrete simulation steps.
///
/// Mirrors the `advance(to:)` internals of `SteppedScene`: it ignores the first
/// observed time (so a scene switch doesn't fast-forward), clamps backwards or
/// stalled clocks, and caps catch-up work per call.
#[derive(Clone, Debug)]
pub struct Stepper {
    last_time: f64,
    accumulator: f64,
    started: bool,
}

impl Stepper {
    pub fn new() -> Self {
        Stepper {
            last_time: 0.0,
            accumulator: 0.0,
            started: false,
        }
    }

    /// Reset the clock so the next [`Stepper::advance`] re-anchors. Called from a
    /// scene's `start()`/`reset()` so a fresh simulation doesn't inherit stale
    /// accumulated time.
    pub fn reset(&mut self) {
        self.last_time = 0.0;
        self.accumulator = 0.0;
        self.started = false;
    }

    /// Advance the clock to `t` and return how many `step()` calls are due.
    ///
    /// `interval` is the simulation seconds between steps (a scene's
    /// `stepInterval`). The first call after a reset returns 0 and just anchors.
    pub fn advance(&mut self, t: f64, interval: f64) -> usize {
        if !self.started {
            self.started = true;
            self.last_time = t;
            self.accumulator = 0.0;
            return 0;
        }
        // Clamp dt: < 0 is the clock going backwards (a scene switch); > 0.25 is
        // a stall we don't want to fast-forward through.
        let dt = (t - self.last_time).clamp(0.0, 0.25);
        self.last_time = t;
        self.accumulator += dt;
        let interval = interval.max(0.0001);
        // Tolerate float drift: accumulating dt (e.g. 0.05 + 0.05) can land a
        // hair under a whole interval and silently swallow a due step.
        const EPS: f64 = 1e-9;
        let mut budget = 12usize; // cap catch-up work per frame
        let mut steps = 0usize;
        while self.accumulator >= interval - EPS && budget > 0 {
            steps += 1;
            self.accumulator -= interval;
            budget -= 1;
        }
        steps
    }
}

impl Default for Stepper {
    fn default() -> Self {
        Stepper::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_advance_anchors_and_returns_zero() {
        let mut s = Stepper::new();
        assert_eq!(s.advance(10.0, 1.0 / 30.0), 0);
    }

    #[test]
    fn accumulates_into_discrete_steps() {
        let mut s = Stepper::new();
        s.advance(0.0, 0.1); // anchor
        assert_eq!(s.advance(0.25, 0.1), 2); // 0.25s / 0.1 = 2 whole steps
        assert_eq!(s.advance(0.30, 0.1), 1); // leftover 0.05 + 0.05 = 0.1 → 1
    }

    #[test]
    fn clamps_long_stall() {
        let mut s = Stepper::new();
        s.advance(0.0, 0.1); // anchor
                             // A 10s jump is clamped to 0.25s → at most 2 steps despite the gap.
        assert_eq!(s.advance(10.0, 0.1), 2);
    }

    #[test]
    fn backwards_clock_yields_no_steps() {
        let mut s = Stepper::new();
        s.advance(5.0, 0.1); // anchor
        assert_eq!(s.advance(4.0, 0.1), 0);
    }

    #[test]
    fn stepper_default_and_reset() {
        let mut s = Stepper::default();
        s.advance(0.0, 0.1);
        assert_eq!(s.advance(0.2, 0.1), 2);

        s.reset();
        // After reset, the first advance should anchor and return 0
        assert_eq!(s.advance(100.0, 0.1), 0);
        assert_eq!(s.advance(100.2, 0.1), 2);
    }
}
