pub mod motors;
pub mod network;
pub mod peripherals;

use crate::grid::standard_grid::StandardGrid;
use crate::messages::{FrequentServerToRobot, NetworkStatus, RobotToServerMessage, SensorData};
use core::time::Duration;
#[cfg(feature = "defmt")]
pub(crate) use defmt::*;
#[cfg(feature = "log")]
pub(crate) use log::*;

/// The different async tasks that run on the robot
#[derive(Copy, Clone, Debug)]
pub enum Task {
    Wifi,
    Motors,
    Peripherals,
}

/// Messages passed between the various tasks
#[derive(Clone)]
pub enum RobotInterTaskMessage {
    FrequentServerToRobot(FrequentServerToRobot),
    ToServer(RobotToServerMessage),
    Sensors(SensorData),
    Grid(StandardGrid),
    NetworkStatus(NetworkStatus, Option<[u8; 4]>),
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
