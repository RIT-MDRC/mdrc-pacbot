#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
#[cfg(feature = "robot")]
pub mod driving;
pub mod grid;
pub mod messages;

pub use pacbot_rs;

use serde::de::DeserializeOwned;
#[cfg(feature = "std")]
use serde::Serialize;

/// [`bincode::serde::encode_to_vec`] with [`bincode::config::standard`]
#[cfg(feature = "std")]
pub fn bin_encode<T: Serialize>(x: T) -> Result<Vec<u8>, bincode::error::EncodeError> {
    bincode::serde::encode_to_vec(x, bincode::config::standard())
}

// pub fn msg_encode<T: Serialize>(x: T) -> Result<Mess>

/// [`bincode::serde::decode_from_slice`] with [`bincode::config::standard`]
pub fn bin_decode<'a, T: DeserializeOwned>(
    bytes: &[u8],
) -> Result<(T, usize), bincode::error::DecodeError> {
    bincode::serde::decode_from_slice(bytes, bincode::config::standard())
}
