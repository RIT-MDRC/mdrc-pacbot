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
pub enum SimPeripheralsError {
    Unknown,
}

impl RobotTask for SimPeripherals {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        self.channels.send_or_drop(message, to)
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        self.channels.send_blocking(message, to).await
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

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error> {
        self.robot
            .read()
            .unwrap()
            .imu_angle
            .map_err(|_| SimPeripheralsError::Unknown)
    }

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error> {
        self.robot.read().unwrap().distance_sensors[index].map_err(|_| SimPeripheralsError::Unknown)
    }

    async fn battery_level(&mut self) -> Result<f32, Self::Error> {
        Ok(1.0)
    }
}
