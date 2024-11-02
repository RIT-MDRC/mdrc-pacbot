//! Contains messages and functionality that are shared between other crates
//!
//! Workspace members (gui_pb, server_pb, sim_pb) have feature std

#![cfg_attr(not(feature = "std"), no_std)]

pub mod constants;
pub mod drive_system;
pub mod driving;
pub mod grid;
pub mod localization;
pub mod messages;
pub mod names;
pub mod pure_pursuit;
pub mod robot_definition;
pub mod robot_display;
#[cfg(feature = "std")]
pub mod threaded_websocket;
mod util;

pub use pacbot_rs;
pub use util::*;
