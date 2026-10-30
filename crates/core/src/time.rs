//! Fixed-step timing.
//!
//! The simulation always advances in steps of exactly `step_seconds`. Each display frame feeds
//! the measured frame time into an accumulator; whole steps are taken out of it and the remainder
//! becomes the interpolation factor `alpha` used when drawing.

/// Default simulation rate: 60 Hz.
pub const FIXED_STEP_SECONDS: f32 = 1.0 / 60.0;
/// The most steps one display frame may run before the backlog is dropped.
pub const MAX_STEPS_PER_FRAME: u32 = 8;
/// Frame times above this are clamped (a debugger pause or a window drag).
pub const MAX_FRAME_SECONDS: f64 = 0.25;
/// Frame times this close to one step are treated as exactly one step (vsync jitter).
pub const SNAP_THRESHOLD_SECONDS: f64 = 0.0002;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStepPlan {
    /// How many fixed updates to run this display frame.
    pub step_count: u32,
    /// How far between the previous and the current simulation state to draw, in 0..1.
    pub alpha: f32,
    /// Simulation time thrown away because the frame needed more than `MAX_STEPS_PER_FRAME`.
    pub dropped_seconds: f64,
}

#[derive(Clone, Debug)]
pub struct FixedStepClock {
    step_seconds: f64,
    max_steps_per_frame: u32,
    accumulator_seconds: f64,
    total_steps: u64,
}

impl Default for FixedStepClock {
    fn default() -> Self {
        FixedStepClock::new(f64::from(FIXED_STEP_SECONDS), MAX_STEPS_PER_FRAME)
    }
}

impl FixedStepClock {
    pub fn new(step_seconds: f64, max_steps_per_frame: u32) -> FixedStepClock {
        assert!(step_seconds > 0.0, "the fixed step must be positive");
        assert!(max_steps_per_frame > 0, "a frame must be allowed at least one step");
        FixedStepClock { step_seconds, max_steps_per_frame, accumulator_seconds: 0.0, total_steps: 0 }
    }

    pub fn step_seconds(&self) -> f32 {
        self.step_seconds as f32
    }

    pub fn total_steps(&self) -> u64 {
        self.total_steps
    }

    /// Adds one display frame's measured time and says how many fixed steps to run.
    pub fn advance(&mut self, frame_seconds: f64) -> FixedStepPlan {
        let mut frame_seconds = frame_seconds.clamp(0.0, MAX_FRAME_SECONDS);
        if (frame_seconds - self.step_seconds).abs() < SNAP_THRESHOLD_SECONDS {
            frame_seconds = self.step_seconds;
        }
        self.accumulator_seconds += frame_seconds;

        let mut step_count = 0;
        while self.accumulator_seconds >= self.step_seconds && step_count < self.max_steps_per_frame {
            self.accumulator_seconds -= self.step_seconds;
            step_count += 1;
        }

        // Spiral-of-death guard: if we are still behind after the cap, drop whole steps of
        // backlog rather than trying to catch up next frame.
        let mut dropped_seconds = 0.0;
        if self.accumulator_seconds >= self.step_seconds {
            let whole_steps_behind = (self.accumulator_seconds / self.step_seconds).floor();
            dropped_seconds = whole_steps_behind * self.step_seconds;
            self.accumulator_seconds -= dropped_seconds;
        }

        self.total_steps += u64::from(step_count);
        FixedStepPlan { step_count, alpha: (self.accumulator_seconds / self.step_seconds) as f32, dropped_seconds }
    }
}

/// Rolling frame statistics for the HUD.
#[derive(Clone, Debug)]
pub struct FrameStats {
    frame_seconds: [f64; 120],
    next: usize,
    filled: usize,
    pub last_step_count: u32,
    pub frames: u64,
    pub dropped_seconds_total: f64,
}

impl Default for FrameStats {
    fn default() -> Self {
        FrameStats {
            frame_seconds: [0.0; 120],
            next: 0,
            filled: 0,
            last_step_count: 0,
            frames: 0,
            dropped_seconds_total: 0.0,
        }
    }
}

impl FrameStats {
    pub fn record(&mut self, frame_seconds: f64, plan: &FixedStepPlan) {
        self.frame_seconds[self.next] = frame_seconds;
        self.next = (self.next + 1) % self.frame_seconds.len();
        self.filled = (self.filled + 1).min(self.frame_seconds.len());
        self.last_step_count = plan.step_count;
        self.dropped_seconds_total += plan.dropped_seconds;
        self.frames += 1;
    }

    pub fn average_frame_milliseconds(&self) -> f64 {
        if self.filled == 0 {
            return 0.0;
        }
        self.frame_seconds[..self.filled].iter().sum::<f64>() / self.filled as f64 * 1000.0
    }

    pub fn worst_frame_milliseconds(&self) -> f64 {
        self.frame_seconds[..self.filled].iter().copied().fold(0.0, f64::max) * 1000.0
    }

    pub fn frames_per_second(&self) -> f64 {
        let average = self.average_frame_milliseconds();
        if average <= 0.0 { 0.0 } else { 1000.0 / average }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f64 = 1.0 / 60.0;

    #[test]
    fn one_step_per_frame_at_the_step_rate() {
        let mut clock = FixedStepClock::default();
        for _ in 0..100 {
            assert_eq!(clock.advance(STEP).step_count, 1);
        }
        assert_eq!(clock.total_steps(), 100);
    }

    #[test]
    fn vsync_jitter_is_snapped_so_steps_never_double_up() {
        let mut clock = FixedStepClock::default();
        for frame in 0..1000 {
            let jitter = if frame % 2 == 0 { 0.00015 } else { -0.00015 };
            assert_eq!(clock.advance(STEP + jitter).step_count, 1, "frame {frame}");
        }
    }

    #[test]
    fn a_fast_display_runs_steps_on_some_frames_only() {
        let mut clock = FixedStepClock::default();
        let steps: u32 = (0..240).map(|_| clock.advance(1.0 / 240.0).step_count).sum();
        // Floating-point accumulation can leave the 60th step a hair short of due.
        assert!((59..=60).contains(&steps), "one second of 240 Hz frames gave {steps} steps");
    }

    #[test]
    fn alpha_is_the_remainder_fraction() {
        let mut clock = FixedStepClock::default();
        let plan = clock.advance(STEP * 1.5);
        assert_eq!(plan.step_count, 1);
        assert!((plan.alpha - 0.5).abs() < 1e-4);
    }

    #[test]
    fn a_long_frame_is_capped_and_the_backlog_dropped() {
        let mut clock = FixedStepClock::default();
        let plan = clock.advance(0.2); // 12 steps' worth
        assert_eq!(plan.step_count, MAX_STEPS_PER_FRAME);
        assert!(plan.dropped_seconds > 0.0);
        assert!(plan.alpha < 1.0);
        // The next normal frame is back to one step.
        assert_eq!(clock.advance(STEP).step_count, 1);
    }

    #[test]
    fn huge_and_negative_frame_times_are_clamped() {
        let mut clock = FixedStepClock::default();
        assert!(clock.advance(30.0).step_count <= MAX_STEPS_PER_FRAME);
        assert_eq!(clock.advance(-1.0).step_count, 0);
    }

    #[test]
    fn simulation_time_matches_wall_time_at_any_frame_rate() {
        for hz in [30.0, 59.0, 60.0, 75.0, 144.0, 240.0] {
            let mut clock = FixedStepClock::default();
            let frames = (hz * 10.0) as usize;
            let steps: u32 = (0..frames).map(|_| clock.advance(1.0 / hz).step_count).sum();
            assert!((steps as i64 - 600).abs() <= 1, "{hz} Hz gave {steps} steps in 10 s");
        }
    }

    #[test]
    fn frame_stats_average() {
        let mut stats = FrameStats::default();
        let plan = FixedStepPlan { step_count: 1, alpha: 0.0, dropped_seconds: 0.0 };
        for _ in 0..10 {
            stats.record(0.02, &plan);
        }
        assert!((stats.frames_per_second() - 50.0).abs() < 1e-6);
    }
}
