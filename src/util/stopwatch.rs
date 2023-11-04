//! Keeps track of time elapsed for a process that may have multiple parts

use crate::util::moving_average::MovingAverage;
use std::time::{Duration, Instant};

pub struct Stopwatch {
    start_time: Option<Instant>,
    segment_times: Vec<Duration>,
    process_times: Vec<Duration>,
    process_moving_average: MovingAverage,
    segment_moving_averages: Vec<MovingAverage>,
}

impl Stopwatch {
    /// Creates a new Stopwatch
    pub fn new(num_segments: usize, default_average: f32) -> Self {
        Stopwatch {
            start_time: None,
            segment_times: Vec::with_capacity(num_segments),
            process_times: Vec::new(),
            process_moving_average: MovingAverage::new(1, default_average),
            segment_moving_averages: (0..num_segments)
                .map(|_| MovingAverage::new(1, default_average))
                .collect(),
        }
    }

    /// Mark the beginning of the process
    pub fn start(&mut self) {
        if self.start_time.is_some() {
            panic!("Stopwatch is already running");
        }
        self.segment_times.clear();
        self.start_time = Some(Instant::now());
    }

    /// Mark a segment of the process
    pub fn mark_segment(&mut self) {
        let start = self.start_time.expect("Stopwatch has not been started");
        let now = Instant::now();
        let duration = now.duration_since(start);

        if let Some(&last_segment_end) = self.segment_times.last() {
            let segment_duration = duration - last_segment_end;
            self.segment_times.push(segment_duration);
        } else {
            self.segment_times.push(duration);
        }

        // Check if the number of segments is consistent
        if self.process_times.len() > 0
            && self.segment_times.len() > self.segment_moving_averages.len()
        {
            panic!("Inconsistent number of segments");
        }
    }

    /// Mark the end of a process
    pub fn end(&mut self) {
        let start = self
            .start_time
            .take()
            .expect("Stopwatch has not been started");
        let duration = Instant::now().duration_since(start);

        // Ensure the number of segments is consistent with previous runs
        if !self.process_times.is_empty()
            && self.segment_times.len() != self.segment_moving_averages.len()
        {
            panic!("Inconsistent number of segments at the end of the process");
        }

        // Update moving averages
        self.process_moving_average
            .add_sample(duration.as_secs_f32());
        for (i, &segment_duration) in self.segment_times.iter().enumerate() {
            self.segment_moving_averages[i].add_sample(segment_duration.as_secs_f32());
        }
        self.process_times.push(duration);
    }

    pub fn average_process_time(&self) -> f32 {
        self.process_moving_average.average()
    }

    pub fn average_segment_times(&self) -> Vec<f32> {
        self.segment_moving_averages
            .iter()
            .map(|ma| ma.average())
            .collect()
    }
}
