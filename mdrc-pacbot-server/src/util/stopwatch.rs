//! Keeps track of time elapsed for a process that may have multiple parts

use crate::util::moving_average::MovingAverage;
use std::time::Instant;

/// Keeps track of time elapsed for a process that may have multiple parts
#[derive(Clone, Debug)]
pub struct Stopwatch {
    num_samples: usize,

    last_start_time: Option<Instant>,
    last_segment_time: Option<Instant>,
    segment: usize,

    process_moving_average: MovingAverage,
    segment_moving_averages: Vec<(String, MovingAverage)>,

    display_name: String,
    ok_time_millis: f32,
    bad_time_millis: f32,
}

impl Stopwatch {
    /// Creates a new Stopwatch
    pub fn new(
        num_samples: usize,
        display_name: String,
        ok_time_millis: f32,
        bad_time_millis: f32,
    ) -> Self {
        Stopwatch {
            num_samples,
            last_start_time: None,
            last_segment_time: None,
            segment: 0,
            process_moving_average: MovingAverage::new(num_samples),
            segment_moving_averages: vec![],
            display_name,
            ok_time_millis,
            bad_time_millis,
        }
    }

    /// Get this stopwatch's name
    pub fn display_name(&self) -> String {
        self.display_name.clone()
    }

    /// Get the amount of time under which the stopwatch is green
    pub fn ok_time_millis(&self) -> f32 {
        self.ok_time_millis
    }

    /// Get the amount of time under which the stopwatch is yellow
    pub fn bad_time_millis(&self) -> f32 {
        self.bad_time_millis
    }

    /// Mark the beginning of the process
    pub fn start(&mut self) {
        let now = Instant::now();
        self.last_start_time = Some(now);
        self.last_segment_time = Some(now);
        self.segment = 0;
    }

    /// Mark a segment of the process
    pub fn mark_segment(&mut self, name: &str) {
        let now = Instant::now();
        if let Some(t) = self.last_segment_time {
            if self.segment_moving_averages.len() < self.segment + 1 {
                // this is the first time through the process
                self.segment_moving_averages
                    .push((name.to_string(), MovingAverage::new(self.num_samples)));
            } else {
                // this is not the first time through the process; if this is the last segment,
                // mark the time
                if self.segment + 1 == self.segment_moving_averages.len() {
                    self.process_moving_average.add_sample(
                        now.duration_since(self.last_start_time.unwrap())
                            .as_secs_f32(),
                    )
                }
            }
            self.segment_moving_averages[self.segment]
                .1
                .add_sample(now.duration_since(t).as_secs_f32());
        }
        self.last_segment_time = Some(now);
        self.segment += 1;
    }

    /// Get the average time it is taking to complete the whole process start -> end
    pub fn average_process_time(&self) -> f32 {
        self.process_moving_average.average()
    }

    /// Get the average time it is taking to complete each segment
    pub fn average_segment_times(&self) -> Vec<(String, f32)> {
        self.segment_moving_averages
            .iter()
            .map(|(s, ma)| (s.to_owned(), ma.average()))
            .collect()
    }
}
