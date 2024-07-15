use crate::send;
use core_pb::driving::motors::RobotMotorsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use defmt::Format;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

pub static MOTORS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> = Channel::new();

pub struct Motors {}

#[derive(Debug, Format)]
pub enum MotorError {}

impl RobotTask for Motors {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        send(message, to).await.map_err(|_| ())
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        MOTORS_CHANNEL.receive().await
    }
}

impl RobotMotorsBehavior for Motors {
    type Error = ();

    fn do_pid(&self) -> bool {
        true
    }

    async fn set_motor_speed(&mut self, index: usize, to: f32) {
        todo!()
    }
}
