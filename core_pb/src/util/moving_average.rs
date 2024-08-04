use core::ops::{Add, Sub};
use core::time::Duration;

pub trait Number: Copy + Add<Output = Self> + Sub<Output = Self> {
    const ZERO: Self;
    const ONE: Self;
    fn div_usize(self, by: usize) -> Self;
}

macro_rules! impl_number {
    ($($t:ty)*) => ($(
        impl Number for $t {
            const ZERO: Self = 0 as $t;
            const ONE: Self = 1 as $t;

            fn div_usize(self, by: usize) -> Self {
                self / (by as $t)
            }
        }
    )*)
}

impl_number!(f32 f64 u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);

impl Number for Duration {
    const ZERO: Self = Self::ZERO;
    const ONE: Self = Self::from_secs(1);

    fn div_usize(self, by: usize) -> Self {
        self / by as u32
    }
}

#[derive(Copy, Clone)]
pub struct MovingAverage<T, const COUNT: usize> {
    measurements: [Option<T>; COUNT],
    idx: usize,
    sum: T,
}

impl<T: Number, const COUNT: usize> MovingAverage<T, COUNT> {
    pub fn new() -> Self {
        Self {
            measurements: [None; COUNT],
            idx: 0,
            sum: T::ZERO,
        }
    }

    pub fn reset(&mut self) {
        self.measurements = [None; COUNT];
        self.idx = 0;
        self.sum = T::ZERO;
    }

    pub fn add(&mut self, new: T) {
        if let Some(oldest) = self.measurements[self.idx] {
            self.sum = self.sum - oldest;
        }
        self.measurements[self.idx] = Some(new);
        self.sum = self.sum + new;
        self.idx += 1;
        if self.idx == COUNT {
            self.idx = 0;
        }
    }

    pub fn sum(&self) -> T {
        self.sum
    }

    pub fn count(&self) -> usize {
        if self.measurements[self.idx].is_some() {
            COUNT
        } else {
            self.idx
        }
    }

    pub fn average(&self) -> T {
        let c = self.count();
        if c == 0 {
            T::ZERO
        } else {
            self.sum.div_usize(c)
        }
    }

    pub fn oldest(&self) -> Option<T> {
        self.measurements[self.idx]
    }

    pub fn latest(&self) -> Option<T> {
        let idx = if self.idx == 0 { COUNT - 1 } else { self.idx };
        self.measurements[idx]
    }
}
