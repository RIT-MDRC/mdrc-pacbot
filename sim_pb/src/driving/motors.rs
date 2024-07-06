use crate::driving::TaskChannels;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};

pub struct SimMotors {
    channels: TaskChannels,
}

impl SimMotors {
    pub fn new(channels: TaskChannels) -> Self {
        Self { channels }
    }
}

#[derive(Debug)]
pub enum SimMotorsError {}

impl RobotTask for SimMotors {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        self.channels.send_message(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        self.channels.receive_message().await
    }
}

impl RobotMotorsBehavior for SimMotors {
    type Error = SimMotorsError;
}
