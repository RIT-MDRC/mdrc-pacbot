use crate::driving::TaskChannels;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};

pub struct SimPeripherals {
    channels: TaskChannels,
}

impl SimPeripherals {
    pub fn new(channels: TaskChannels) -> Self {
        Self { channels }
    }
}

#[derive(Debug)]
pub enum SimPeripheralsError {}

impl RobotTask for SimPeripherals {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        self.channels.send_message(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        self.channels.receive_message().await
    }
}

impl RobotPeripheralsBehavior for SimPeripherals {
    type Error = SimPeripheralsError;
}
