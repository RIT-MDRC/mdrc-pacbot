use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Stores new values and updates its output after a delay
pub struct DelayedValue<T: PartialEq> {
    delay: Duration,
    next: VecDeque<(Instant, T)>,
    output: T,
}

impl<T: PartialEq> DelayedValue<T> {
    pub fn new(delay: Duration, initial: T) -> Self {
        Self {
            delay,
            next: VecDeque::new(),
            output: initial,
        }
    }

    pub fn register(&mut self, value: T) {
        self.next.push_back((Instant::now(), value))
    }

    pub fn get(&mut self) -> &T {
        loop {
            let mut remove = false;
            if let Some((i, _)) = self.next.front() {
                if i.elapsed() > self.delay {
                    remove = true;
                }
            }
            if remove {
                if let Some((_, v)) = self.next.pop_front() {
                    self.output = v;
                }
            } else {
                break;
            }
        }

        &self.output
    }
}

/// Represents a value (usually a sensor) that should not update more often than the given frequency
pub struct LimitedRefreshRate<T> {
    min_time_to_update: Duration,
    next: Option<(Instant, T)>,
    last_update: Instant,
    output: T,
}

impl<T> LimitedRefreshRate<T> {
    pub fn new(min_time_to_update: Duration, initial: T) -> Self {
        Self {
            min_time_to_update,
            next: None,
            last_update: Instant::now(),
            output: initial,
        }
    }

    pub fn register(&mut self, value: T) {
        self.next = Some((Instant::now(), value))
    }

    pub fn get(&mut self) -> &T {
        if let Some((next_t, next)) = self.next.take() {
            if next_t - self.last_update > self.min_time_to_update {
                self.output = next;
            } else {
                self.next = Some((next_t, next))
            }
        }

        &self.output
    }
}
