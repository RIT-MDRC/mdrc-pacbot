use crate::util::moving_average::MovingAverage;
use crate::util::{ColoredStatus, CrossPlatformInstant};
use core::time::Duration;

#[derive(Copy, Clone)]
pub struct UtilizationMonitor<const C: usize, I> {
    warn_amount: f32,
    error_amount: f32,

    last_start: I,
    last_stop: I,
    active_durations: MovingAverage<Duration, C>,
    inactive_durations: MovingAverage<Duration, C>,
    start_to_start_durations: MovingAverage<Duration, C>,
}

impl<const C: usize, I: CrossPlatformInstant + Default> Default for UtilizationMonitor<C, I> {
    fn default() -> Self {
        Self::new(0.8, 0.9)
    }
}

impl<const C: usize, I: CrossPlatformInstant + Default> UtilizationMonitor<C, I> {
    pub fn new(warn_amount: f32, error_amount: f32) -> Self {
        Self {
            warn_amount,
            error_amount,

            last_start: I::default(),
            last_stop: I::default(),
            active_durations: MovingAverage::new(),
            inactive_durations: MovingAverage::new(),
            start_to_start_durations: MovingAverage::new(),
        }
    }

    pub fn start(&mut self) {
        let now = I::default();
        if let Some(t) = now.checked_duration_since(self.last_stop) {
            self.inactive_durations.add(t)
        }
        if let Some(t) = now.checked_duration_since(self.last_start) {
            self.start_to_start_durations.add(t)
        }
        self.last_start = now;
    }

    pub fn stop(&mut self) {
        let now = I::default();
        if let Some(t) = now.checked_duration_since(self.last_start) {
            self.active_durations.add(t)
        }
        self.last_stop = now;
    }

    pub fn reset(&mut self) {
        self.last_start = I::default();
        self.last_stop = I::default();
        self.active_durations = MovingAverage::new();
        self.inactive_durations = MovingAverage::new();
        self.start_to_start_durations = MovingAverage::new();
    }

    pub fn utilization(&self) -> f32 {
        self.active_durations.sum().as_secs_f32()
            / (self.active_durations.sum() + self.inactive_durations.sum()).as_secs_f32()
    }

    pub fn status(&self) -> ColoredStatus {
        let util = self.utilization();
        if util >= self.error_amount {
            ColoredStatus::Error(Some(format!(
                "{:.1?}% >= {:.1}%",
                util * 100.0,
                self.error_amount
            )))
        } else if util >= self.warn_amount {
            ColoredStatus::Warn(Some(format!(
                "{:.1?}% >= {:.1}%",
                util * 100.0,
                self.warn_amount
            )))
        } else {
            ColoredStatus::Ok(Some(format!("{:.1?}%", util * 100.0)))
        }
    }

    pub fn active_time(&self) -> Duration {
        self.active_durations.average()
    }

    pub fn hz(&self) -> f32 {
        1.0 / self.start_to_start_durations.average().as_secs_f32()
    }

    pub fn inactive_time(&self) -> Duration {
        self.inactive_durations.average()
    }

    pub fn last_active_duration(&self) -> Option<Duration> {
        self.active_durations.latest()
    }

    pub fn last_inactive_duration(&self) -> Option<Duration> {
        self.inactive_durations.latest()
    }
}
