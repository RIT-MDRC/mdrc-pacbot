use core::time::Duration;
use ecolor::Color32;

pub mod average_rate;
pub mod moving_average;
pub mod stopwatch;
pub mod utilization;

pub const TRANSLUCENT_GREEN_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 50, 0, 50);
pub const TRANSLUCENT_YELLOW_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 50, 0, 50);
pub const TRANSLUCENT_RED_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 0, 0, 50);

#[cfg(feature = "std")]
#[derive(Clone)]
pub enum ColoredStatus {
    Ok(Option<String>),
    Warn(Option<String>),
    Error(Option<String>),
    NotApplicable(Option<String>),
}

#[cfg(feature = "std")]
impl ColoredStatus {
    pub fn to_color32(&self) -> Color32 {
        match self {
            ColoredStatus::Ok(_) => TRANSLUCENT_GREEN_COLOR,
            ColoredStatus::Warn(_) => TRANSLUCENT_YELLOW_COLOR,
            ColoredStatus::Error(_) => TRANSLUCENT_RED_COLOR,
            ColoredStatus::NotApplicable(_) => Color32::TRANSPARENT,
        }
    }

    pub fn to_color32_solid(&self) -> Color32 {
        match self {
            ColoredStatus::Ok(_) => Color32::GREEN,
            ColoredStatus::Warn(_) => Color32::YELLOW,
            ColoredStatus::Error(_) => Color32::RED,
            ColoredStatus::NotApplicable(_) => Color32::GRAY,
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ColoredStatus::Ok(_) => egui_phosphor::regular::CHECK,
            ColoredStatus::Warn(_) => egui_phosphor::regular::WARNING,
            ColoredStatus::Error(_) => egui_phosphor::regular::X,
            ColoredStatus::NotApplicable(_) => "",
        }
    }

    pub fn severity(&self) -> usize {
        match self {
            ColoredStatus::Ok(_) => 1,
            ColoredStatus::Warn(_) => 2,
            ColoredStatus::Error(_) => 3,
            ColoredStatus::NotApplicable(_) => 0,
        }
    }

    pub fn message(&self) -> Option<String> {
        match self {
            ColoredStatus::Ok(s) => s.clone(),
            ColoredStatus::Warn(s) => s.clone(),
            ColoredStatus::Error(s) => s.clone(),
            ColoredStatus::NotApplicable(s) => s.clone(),
        }
    }
}

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
