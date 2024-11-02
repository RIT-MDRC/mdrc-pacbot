//! Some utility enums and functions

use core::time::Duration;
#[cfg(feature = "std")]
use ecolor::Color32;
use serde::de::DeserializeOwned;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub mod average_rate;
pub mod moving_average;
pub mod stopwatch;
pub mod utilization;

#[cfg(feature = "std")]
pub const TRANSLUCENT_GREEN_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 50, 0, 50);
#[cfg(feature = "std")]
pub const TRANSLUCENT_YELLOW_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 50, 0, 50);
#[cfg(feature = "std")]
pub const TRANSLUCENT_RED_COLOR: Color32 = Color32::from_rgba_premultiplied(50, 0, 0, 50);

#[cfg(feature = "std")]
#[derive(Clone, Debug, Serialize, Deserialize)]
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

    #[cfg(feature = "egui-phosphor")]
    pub fn icon(&self) -> &str {
        match self {
            ColoredStatus::Ok(_) => egui_phosphor::regular::CHECK,
            ColoredStatus::Warn(_) => egui_phosphor::regular::WARNING,
            ColoredStatus::Error(_) => egui_phosphor::regular::X,
            ColoredStatus::NotApplicable(_) => egui_phosphor::regular::MINUS,
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

/// Instant functionality that may be implemented differently on different platforms
pub trait CrossPlatformInstant: Copy {
    fn now() -> Self;

    fn elapsed(&self) -> Duration;

    fn checked_duration_since(&self, other: Self) -> Option<Duration>;
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[derive(Copy, Clone)]
pub struct StdInstant(std::time::Instant);

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
impl CrossPlatformInstant for StdInstant {
    fn now() -> Self {
        Self(std::time::Instant::now())
    }

    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn checked_duration_since(&self, other: Self) -> Option<Duration> {
        self.0.checked_duration_since(other.0)
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Copy, Clone)]
pub struct WebTimeInstant(web_time::Instant);

#[cfg(target_arch = "wasm32")]
impl CrossPlatformInstant for WebTimeInstant {
    fn now() -> Self {
        Self(web_time::Instant::now())
    }

    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn checked_duration_since(&self, other: Self) -> Option<Duration> {
        self.0.checked_duration_since(other.0)
    }
}
#[cfg(target_arch = "wasm32")]
#[macro_export]
/// for WASM, prints the message to the javascript developer console, otherwise uses [`log::info!`]
macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
/// for WASM, prints the message to the javascript developer console, otherwise uses [`log::error!`]
macro_rules! console_error {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
/// for WASM, prints the message to the javascript developer console, otherwise uses `log::info!`
macro_rules! console_log {
    ($($t:tt)*) => (log::info!($($t)*))
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
/// for WASM, prints the message to the javascript developer console, otherwise uses `log::error!`
macro_rules! console_error {
    ($($t:tt)*) => (log::error!($($t)*))
}

/// [`bincode::serde::encode_to_vec`] with [`bincode::config::standard`]
#[cfg(feature = "std")]
pub fn bin_encode<T: Serialize>(x: T) -> Result<Vec<u8>, bincode::error::EncodeError> {
    bincode::serde::encode_to_vec(x, bincode::config::standard())
}

/// [`bincode::serde::decode_from_slice`] with [`bincode::config::standard`]
pub fn bin_decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, bincode::error::DecodeError> {
    bincode::serde::decode_from_slice(bytes, bincode::config::standard()).map(|x| x.0)
}
