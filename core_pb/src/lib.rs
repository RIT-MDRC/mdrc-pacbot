#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
pub mod drive_system;
#[allow(async_fn_in_trait)]
pub mod driving;
pub mod grid;
pub mod localization;
pub mod messages;
pub mod names;
pub mod pure_pursuit;
pub mod region_localization;
pub mod robot_definition;
pub mod robot_display;
#[cfg(feature = "std")]
pub mod threaded_websocket;
pub mod util;

use core::fmt::Debug;

pub use pacbot_rs;

#[cfg(feature = "std")]
use crate::threaded_websocket::TextOrT;
use serde::de::DeserializeOwned;
#[cfg(feature = "std")]
use serde::Serialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// [`bincode::serde::encode_to_vec`] with [`bincode::config::standard`]
#[cfg(feature = "std")]
pub fn bin_encode<T: Serialize + Debug>(
    _first: bool,
    x: TextOrT<T>,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    match x {
        TextOrT::Bytes(b) => Ok(b),
        TextOrT::T(t) => bincode::serde::encode_to_vec(t, bincode::config::standard()),
        _ => unimplemented!(),
    }
}

/// [`bincode::serde::decode_from_slice`] with [`bincode::config::standard`]
#[cfg(feature = "std")]
pub fn bin_decode<T: DeserializeOwned + Debug>(
    _first: bool,
    bytes: &[u8],
) -> Result<Vec<TextOrT<T>>, bincode::error::DecodeError> {
    Ok(vec![TextOrT::T(
        bincode::serde::decode_from_slice(bytes, bincode::config::standard()).map(|x| x.0)?,
    )])
}

/// [`bincode::serde::decode_from_slice`] with [`bincode::config::standard`]
pub fn bin_decode_single<T: DeserializeOwned + Debug>(
    bytes: &[u8],
) -> Result<T, bincode::error::DecodeError> {
    bincode::serde::decode_from_slice(bytes, bincode::config::standard()).map(|x| x.0)
}
