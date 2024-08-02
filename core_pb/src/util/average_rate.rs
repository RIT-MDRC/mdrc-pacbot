use crate::util::CrossPlatformInstant;
use core::time::Duration;

#[derive(Copy, Clone)]
pub struct AverageRate<const C: usize, I: CrossPlatformInstant + Default> {
    last_instant: Option<I>,
    durations: [Option<Duration>; C],
    idx: usize,
    current_sum: u128,
    forward: bool,
}

#[allow(dead_code)]
impl<const C: usize, I: CrossPlatformInstant + Default> AverageRate<C, I> {
    pub fn new() -> Self {
        Self {
            last_instant: None,
            durations: [None; C],
            idx: 0,
            current_sum: 0,
            forward: true,
        }
    }

    pub fn reset(&mut self) {
        self.last_instant = None;
        self.durations = [None; C];
        self.idx = 0;
        self.current_sum = 0;
        self.forward = true;
    }

    pub fn tick(&mut self, forward: bool) {
        let now = I::default();

        if self.forward != forward {
            self.reset();
            self.forward = forward;
        }

        if let Some(last_instant) = self.last_instant {
            if let Some(elapsed) = now.checked_duration_since(last_instant) {
                if let Some(old_elapsed) = self.durations[self.idx] {
                    self.current_sum -= old_elapsed.as_micros();
                }
                self.current_sum += elapsed.as_micros();
                self.durations[self.idx] = Some(elapsed);
                self.idx += 1;
                if self.idx >= C {
                    self.idx = 0;
                }
            }
        }

        self.last_instant = Some(now);
    }

    pub fn average(&self) -> u128 {
        let count = self.durations.iter().filter(|c| c.is_some()).count() as u128;
        if count == 0 {
            0
        } else {
            let mut avg = self.current_sum / count;
            // if the time since the last tick is larger than the average, incorporate it
            if let Some(elapsed) = I::default().checked_duration_since(self.last_instant.unwrap()) {
                let elapsed = elapsed.as_micros();
                if elapsed > avg {
                    avg = ((avg * count) + elapsed) / (count + 1);
                }
            }
            avg
        }
    }

    pub fn forward(&self) -> bool {
        self.forward
    }

    pub fn signed_ticks_per_second(&self) -> f32 {
        let avg = self.average() as f32;
        if avg == 0.0
            || self
                .last_instant
                .map(|t| t.elapsed().as_micros())
                .unwrap_or(0)
                > 50_000
        {
            return 0.0;
        }
        let tps = if self.forward {
            1_000_000.0 / self.average() as f32
        } else {
            -1_000_000.0 / self.average() as f32
        };
        if tps < 5.0 && tps > -5.0 {
            0.0
        } else {
            tps
        }
    }
}
