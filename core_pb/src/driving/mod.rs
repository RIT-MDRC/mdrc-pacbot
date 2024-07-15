pub mod motors;
pub mod network;
pub mod peripherals;

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
#[derive(Copy, Clone)]
pub enum RobotInterTaskMessage {
    TargetVelocity(Vector2<f32>, f32),
}

pub trait RobotTask {
    /// Send a message to all other tasks
    ///
    /// If the receiver's buffer is full, returns Err(())
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()>;

    /// Receive a message from other tasks; may be cancelled
    async fn receive_message(&mut self) -> RobotInterTaskMessage;
}
