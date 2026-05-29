/// Simulation clock and time controls.
///
/// `time` advances with the wall clock when not paused, scaled by `multiplier`.
/// Orbital and spin animations read from `time` (never from `Instant`), so pause
/// and time-scaling work uniformly across the scene.
#[derive(Debug, Clone)]
pub struct SimState {
    /// Abstract simulation time in seconds. Reset to 0 at startup.
    pub time: f64,
    /// Wall-clock-to-sim-time ratio. 1.0 = realtime, 2.0 = twice as fast.
    pub multiplier: f64,
    /// When true, `advance` is a no-op.
    pub is_paused: bool,
}

impl SimState {
    /// Create a new simulation clock at t=0, 1x speed, running.
    pub fn new() -> Self {
        Self {
            time: 0.0,
            multiplier: 1.0,
            is_paused: false,
        }
    }

    /// Advance simulation time by the given real-time delta, if not paused.
    pub fn advance(&mut self, dt: web_time::Duration) {
        if !self.is_paused {
            self.time += dt.as_secs_f64() * self.multiplier;
        }
    }

    /// Flip the paused flag.
    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        tracing::info!("Simulation paused: {}", self.is_paused);
    }

    /// Double the time multiplier.
    pub fn double_speed(&mut self) {
        self.multiplier *= 2.0;
        tracing::info!(
            "Sim time mult: {}x ({} days/s)",
            self.multiplier,
            self.sim_days_per_clock_sec()
        );
    }

    /// Halve the time multiplier.
    pub fn halve_speed(&mut self) {
        self.multiplier /= 2.0;
        tracing::info!(
            "Sim time mult: {}x ({} days/s)",
            self.multiplier,
            self.sim_days_per_clock_sec()
        );
    }

    /// Reset the multiplier to 1x (realtime).
    pub fn reset_speed(&mut self) {
        self.multiplier = 1.0;
        tracing::info!("Sim time mult reset: {}x", self.multiplier);
    }

    /// Returns how many simulation days pass for every second in real/wall-clock time.
    pub const fn sim_days_per_clock_sec(&self) -> f64 {
        self.multiplier / crate::orbital::SEC_PER_DAY
    }

    /// Jump simulation time to midnight (0:00 UT) of the given Gregorian date.
    pub fn jump_to_date(&mut self, year: i32, month: u8, day: u8) {
        self.time =
            crate::orbital::jde_to_sim_time(crate::orbital::gregorian_to_jde(year, month, day));
    }

    /// Set sim time so that for every wall clock second, the sim time advances by `days`.
    pub fn set_sim_days_per_sec(&mut self, days: f64) {
        self.multiplier = crate::orbital::SEC_PER_DAY * days;
        tracing::info!(
            "Sim time mult: {}x ({} days/s)",
            self.multiplier,
            self.sim_days_per_clock_sec()
        );
    }
}

impl Default for SimState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_time::Duration;

    #[test]
    fn advance_scales_by_multiplier() {
        let mut s = SimState::new();
        s.multiplier = 2.0;
        s.advance(Duration::from_secs(1));
        assert!((s.time - 2.0).abs() < 1e-9, "time = {}", s.time);
    }

    #[test]
    fn advance_is_noop_when_paused() {
        let mut s = SimState::new();
        s.is_paused = true;
        s.advance(Duration::from_secs(5));
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn toggle_pause_flips_flag() {
        let mut s = SimState::new();
        assert!(!s.is_paused);
        s.toggle_pause();
        assert!(s.is_paused);
        s.toggle_pause();
        assert!(!s.is_paused);
    }

    #[test]
    fn speed_controls() {
        let mut s = SimState::new();
        s.double_speed();
        s.double_speed();
        assert_eq!(s.multiplier, 4.0);
        s.halve_speed();
        assert_eq!(s.multiplier, 2.0);
        s.reset_speed();
        assert_eq!(s.multiplier, 1.0);
    }

    #[test]
    fn jump_to_date_round_trips() {
        let mut s = SimState::new();
        s.jump_to_date(2026, 5, 29);
        let jde = crate::orbital::sim_time_to_jde(s.time);
        assert_eq!(crate::orbital::jde_to_gregorian(jde), (2026, 5, 29));
    }

    #[test]
    fn set_sim_days_per_sec_matches_readback() {
        let mut s = SimState::new();
        s.set_sim_days_per_sec(30.0);
        assert!((s.sim_days_per_clock_sec() - 30.0).abs() < 1e-9);
    }
}
