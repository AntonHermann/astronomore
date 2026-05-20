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
        tracing::info!("Sim time mult: {}x", self.multiplier);
    }

    /// Halve the time multiplier.
    pub fn halve_speed(&mut self) {
        self.multiplier /= 2.0;
        tracing::info!("Sim time mult: {}x", self.multiplier);
    }

    /// Reset the multiplier to 1x (realtime).
    pub fn reset_speed(&mut self) {
        self.multiplier = 1.0;
        tracing::info!("Sim time mult reset: {}x", self.multiplier);
    }
}

impl Default for SimState {
    fn default() -> Self {
        Self::new()
    }
}
