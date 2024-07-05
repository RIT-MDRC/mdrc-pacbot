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
            if let Some((i, v)) = self.next.pop_front() {
                if i.elapsed() > self.delay {
                    self.output = v;
                    remove = true;
                }
            }
            if !remove {
                break;
            }
        }

        &self.output
    }
}
