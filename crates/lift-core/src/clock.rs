#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperimentClock {
    pub t: f64,
    pub dt: f64,
    pub running: bool,
    pub duration: f64,
}

impl ExperimentClock {
    pub fn new(duration: f64) -> Self {
        Self {
            t: 0.0,
            dt: 1.0 / 60.0,
            running: true,
            duration,
        }
    }

    pub fn step(&mut self) {
        if !self.running {
            return;
        }
        self.t += self.dt;
        if self.t >= self.duration {
            self.t = self.duration;
            self.running = false;
        }
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
        self.running = true;
    }
}

impl Default for ExperimentClock {
    fn default() -> Self {
        Self::new(3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_13_clock_stops_at_duration() {
        let mut clock = ExperimentClock::new(3.0);
        for _ in 0..(60 * 3 + 10) {
            clock.step();
        }
        assert_eq!(clock.t, 3.0);
        assert!(!clock.running);
    }

    #[test]
    fn default_clock_uses_spec_values() {
        let clock = ExperimentClock::default();
        assert_eq!(clock.dt, 1.0 / 60.0);
        assert_eq!(clock.duration, 3.0);
        assert_eq!(clock.t, 0.0);
        assert!(clock.running);
    }

    #[test]
    fn clock_does_not_exceed_duration() {
        let mut clock = ExperimentClock::new(0.05);
        for _ in 0..1000 {
            clock.step();
        }
        assert!(clock.t <= 0.05);
        assert_eq!(clock.t, 0.05);
    }

    #[test]
    fn paused_clock_does_not_advance() {
        let mut clock = ExperimentClock::new(3.0);
        clock.running = false;
        clock.step();
        assert_eq!(clock.t, 0.0);
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut clock = ExperimentClock::new(3.0);
        for _ in 0..30 {
            clock.step();
        }
        clock.reset();
        assert_eq!(clock.t, 0.0);
        assert!(clock.running);
    }
}
