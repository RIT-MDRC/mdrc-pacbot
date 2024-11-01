pub mod motors;
pub mod network;
pub mod peripherals;

use crate::messages::{
    FrequentServerToRobot, NetworkStatus, RobotToServerMessage, SensorData, Task,
};
use core::time::Duration;
#[cfg(feature = "defmt")]
pub(crate) use defmt::*;
#[cfg(feature = "log")]
pub(crate) use log::*;

/// Messages passed between the various tasks
#[derive(Clone)]
pub enum RobotInterTaskMessage {
    FrequentServerToRobot(FrequentServerToRobot),
    ToServer(RobotToServerMessage),
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
