#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
pub mod drive_system;
#[cfg(feature = "robot")]
#[allow(async_fn_in_trait)]
pub mod driving;
pub mod grid;
pub mod messages;
pub mod names;
pub mod robot_definition;
#[cfg(feature = "std")]
pub mod threaded_websocket;

pub use pacbot_rs;

use serde::de::DeserializeOwned;
#[cfg(feature = "std")]
use serde::Serialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[macro_export]
/// for WASM, prints the message to the javascript developer console, otherwise uses `println`
///
/// Requires `use crate::log`
macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (println!($($t)*))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
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
