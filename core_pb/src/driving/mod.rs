pub mod motors;
pub mod network;
pub mod peripherals;

#[cfg(feature = "defmt")]
pub use defmt::*;
#[cfg(feature = "log")]
pub use log::*;

use core::future::Future;

#[derive(Copy, Clone, Debug)]
pub enum Task {
    Wifi,
    Motors,
    Peripherals,
}

/// Messages passed between the various tasks
#[derive(Copy, Clone)]
pub enum RobotInterTaskMessage {}

pub trait RobotTask {
    /// Send a message to all other tasks
    ///
    /// If the receiver's buffer is full, returns Err(())
    fn send_message(
        &mut self,
        message: RobotInterTaskMessage,
        to: Task,
    ) -> impl Future<Output = Result<(), ()>>;

    /// Receive a message from other tasks; may be cancelled
    fn receive_message(&mut self) -> impl Future<Output = RobotInterTaskMessage>;
}
