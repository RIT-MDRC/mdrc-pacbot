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
