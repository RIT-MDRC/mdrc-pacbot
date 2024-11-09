pub mod motors;
pub mod network;
pub mod peripherals;

use crate::messages::{
    FrequentServerToRobot, NetworkStatus, RobotToServerMessage, SensorData, ServerToRobotMessage,
    Task,
};
use core::time::Duration;
use portable_atomic::{AtomicBool, AtomicF32, AtomicI32, AtomicI8};

#[cfg(feature = "defmt")]
pub(crate) use defmt::*;
#[cfg(feature = "log")]
pub(crate) use log::*;

/// Messages passed between the various tasks
#[derive(Clone)]
pub enum RobotInterTaskMessage {
    FrequentServerToRobot(FrequentServerToRobot),
    ToServer(RobotToServerMessage),
    FromServer(ServerToRobotMessage),
    Sensors(SensorData),
    NetworkStatus(NetworkStatus, Option<[u8; 4]>),
    Utilization(f32, Task),
    ResetAngle,
}

/// Functionality that all tasks must support
pub trait RobotTaskMessenger {
    /// Send a message to the given task
    ///
    /// If the receiver's buffer is full, drops the message and returns false
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool;

    /// Send a message to the given task
    ///
    /// Blocks until the receiver's buffer has space
    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task);

    /// Receive a message from other tasks; may be cancelled
    async fn receive_message(&mut self) -> RobotInterTaskMessage;

    /// Receive a message from other tasks
    ///
    /// If timeout has passed, return None
    async fn receive_message_timeout(&mut self, timeout: Duration)
        -> Option<RobotInterTaskMessage>;
}

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_OPTS_BOOL: [AtomicBool; 8] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_OPTS_F32: [AtomicF32; 4] = [
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_OPTS_I32: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_OPTS_I8: [AtomicI8; 4] = [
    AtomicI8::new(0),
    AtomicI8::new(0),
    AtomicI8::new(0),
    AtomicI8::new(0),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_INDICATOR_BOOL: [AtomicBool; 8] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_INDICATOR_F32: [AtomicF32; 4] = [
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
    AtomicF32::new(0.0),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_INDICATOR_I32: [AtomicI32; 4] = [
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
    AtomicI32::new(0),
];

#[deprecated = "Extra options should only be used for temporary testing"]
pub static EXTRA_INDICATOR_I8: [AtomicI8; 4] = [
    AtomicI8::new(0),
    AtomicI8::new(0),
    AtomicI8::new(0),
    AtomicI8::new(0),
];
