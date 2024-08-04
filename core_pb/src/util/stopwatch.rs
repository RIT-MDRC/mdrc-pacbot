use crate::util::moving_average::MovingAverage;
use crate::util::utilization::UtilizationMonitor;
use crate::util::{ColoredStatus, CrossPlatformInstant};
use core::time::Duration;

pub struct Stopwatch<const SEGMENTS: usize, const WINDOW: usize, I> {
    name: &'static str,
    start_time: I,
    last: I,
    segments: [(Option<&'static str>, MovingAverage<Duration, WINDOW>); SEGMENTS],
    idx: usize,

    utilization_monitor: UtilizationMonitor<WINDOW, I>,

    warn_time: Duration,
    error_time: Duration,
}

impl<const SEGMENTS: usize, const WINDOW: usize, I: CrossPlatformInstant + Default>
    Stopwatch<SEGMENTS, WINDOW, I>
{
    pub fn new(
        name: &'static str,
        warn_time: Duration,
        error_time: Duration,
        warn_percent: f32,
        error_percent: f32,
    ) -> Self {
        Self {
            name,
            start_time: I::default(),
            last: I::default(),
            segments: [(None, MovingAverage::new()); SEGMENTS],
            idx: 0,

            utilization_monitor: UtilizationMonitor::new(warn_percent, error_percent),

            warn_time,
            error_time,
        }
    }

    pub fn start(&mut self) {
        self.idx = 0;
        self.start_time = I::default();
        self.last = I::default();
        self.utilization_monitor.start();
    }

    pub fn mark_completed(&mut self, segment: &'static str) -> Result<(), &'static str> {
        let now = I::default();
        let last = self.last;
        self.last = now;
        if let Some(duration) = now.checked_duration_since(last) {
            if let Some(name) = self.segments[self.idx].0 {
                if name != segment {
                    return Err("Stopwatch marked segment didn't match next segment name. Ensure that the Stopwatch has enough capacity, and that all segments occur in the same order every time.");
                }
            } else {
                self.segments[self.idx].0 = Some(segment);
            }

            self.segments[self.idx].1.add(duration);

            self.idx = if self.idx + 1 == SEGMENTS {
                // whole process is finished
                self.utilization_monitor.stop();
                0
            } else {
                self.idx + 1
            };
        }
        Ok(())
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn segment_averages(&self) -> [(Option<&'static str>, Duration); SEGMENTS] {
        self.segments.map(|(n, t)| (n, t.average()))
    }

    pub fn process_average(&self) -> Duration {
        self.utilization_monitor.active_time()
    }

    pub fn utilization(&self) -> &UtilizationMonitor<WINDOW, I> {
        &self.utilization_monitor
    }

    #[cfg(feature = "std")]
    pub fn status(&self) -> ColoredStatus {
        let avg = self.process_average();
        let sw_status = if avg >= self.error_time {
            ColoredStatus::Error(Some(format!("{:.2?} >= {:.2?}", avg, self.error_time)))
        } else if avg >= self.warn_time {
            ColoredStatus::Warn(Some(format!("{:.2?} >= {:.2?}", avg, self.warn_time)))
        } else {
            ColoredStatus::Ok(Some(format!("{:.2?}", avg)))
        };
        let util_status = self.utilization_monitor.status();
        if sw_status.severity() >= util_status.severity() {
            sw_status
        } else {
            util_status
        }
    }
}
