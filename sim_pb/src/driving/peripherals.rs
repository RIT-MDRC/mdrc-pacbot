use crate::driving::{SimRobot, TaskChannels};
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use embedded_graphics::mock_display::MockDisplay;
use embedded_graphics::pixelcolor::BinaryColor;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct SimPeripherals {
    robot: Arc<RwLock<SimRobot>>,
    channels: TaskChannels,
}

impl SimPeripherals {
    pub fn new(robot: Arc<RwLock<SimRobot>>, channels: TaskChannels) -> Self {
        Self { robot, channels }
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

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        self.channels.receive_message_timeout(timeout).await
    }
}

impl RobotPeripheralsBehavior for SimPeripherals {
    type Display = MockDisplay<BinaryColor>;
    type Error = SimPeripheralsError;

    fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display),
    {
        let mut robot = self.robot.write().unwrap();
        draw(&mut robot.display);
    }

    async fn flip_screen(&mut self) {
        self.robot.write().unwrap().display_ready = true;
    }
}
