pub mod motors;
pub mod network;
pub mod peripherals;

use crate::messages::RobotToServerMessage;
use core::time::Duration;
#[cfg(feature = "defmt")]
pub use defmt::*;
#[cfg(feature = "log")]
pub use log::*;
use nalgebra::Vector2;

#[derive(Copy, Clone, Debug)]
pub enum Task {
    Wifi,
    Motors,
    Peripherals,
}

/// Messages passed between the various tasks
#[derive(Clone)]
pub enum RobotInterTaskMessage {
    MotorConfig([[usize; 2]; 3]),
    ToServer(RobotToServerMessage),
    PwmOverride([[Option<u16>; 2]; 3]),
    TargetVelocity(Vector2<f32>, f32),
}

pub trait RobotTask {
    /// Send a message to all other tasks
    ///
    /// If the receiver's buffer is full, returns Err(())
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()>;

    /// Receive a message from other tasks; may be cancelled
    async fn receive_message(&mut self) -> RobotInterTaskMessage;

    /// Receive a message from other tasks
    ///
    /// If timeout has passed, return None
    async fn receive_message_timeout(&mut self, timeout: Duration)
        -> Option<RobotInterTaskMessage>;
}
