use core::time::Duration;

pub mod average_rate;

pub trait CrossPlatformInstant: Copy {
    fn elapsed(&self) -> Duration;

    fn checked_duration_since(&self, other: Self) -> Option<Duration>;
}

#[cfg(feature = "std")]
#[derive(Copy, Clone)]
pub struct StdInstant(std::time::Instant);

#[cfg(feature = "std")]
impl Default for StdInstant {
    fn default() -> Self {
        Self(std::time::Instant::now())
    }
}

#[cfg(feature = "std")]
impl CrossPlatformInstant for StdInstant {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn checked_duration_since(&self, other: Self) -> Option<Duration> {
        self.0.checked_duration_since(other.0)
    }
}
