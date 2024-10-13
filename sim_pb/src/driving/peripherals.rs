use crate::driving::{SimRobot, TaskChannels};
use core_pb::constants::{ROBOT_DISPLAY_HEIGHT, ROBOT_DISPLAY_WIDTH};
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::messages::RobotButton;
use core_pb::util::StdInstant;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::Pixel;
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
    type Display = SimDisplay;
    type Instant = StdInstant;
    type Error = SimPeripheralsError;

    fn draw_display<F>(&mut self, draw: F) -> Result<(), SimPeripheralsError>
    where
        F: FnOnce(&mut Self::Display) -> Result<(), SimPeripheralsError>,
    {
        let mut robot = self.robot.write().unwrap();
        draw(&mut robot.display)?;
        Ok(())
    }

    async fn flip_screen(&mut self) {
        self.robot.write().unwrap().display_updated = true;
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

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)> {
        self.robot.write().unwrap().button_events.pop_front()
    }

    async fn read_joystick(&mut self) -> Option<(f32, f32)> {
        self.robot.read().unwrap().joystick
    }
}

#[derive(Clone)]
pub struct SimDisplay {
    pub pixels: [[bool; ROBOT_DISPLAY_WIDTH]; ROBOT_DISPLAY_HEIGHT],
}

impl Default for SimDisplay {
    fn default() -> Self {
        Self {
            pixels: [[false; ROBOT_DISPLAY_WIDTH]; ROBOT_DISPLAY_HEIGHT],
        }
    }
}

impl OriginDimensions for SimDisplay {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}

impl DrawTarget for SimDisplay {
    type Color = BinaryColor;
    type Error = SimPeripheralsError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if 0 <= coord.x
                && coord.x < ROBOT_DISPLAY_WIDTH as i32
                && 0 <= coord.y
                && coord.y < ROBOT_DISPLAY_HEIGHT as i32
            {
                self.pixels[coord.y as usize][coord.x as usize] = color == BinaryColor::On;
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pixels = [[color == BinaryColor::On; ROBOT_DISPLAY_WIDTH]; ROBOT_DISPLAY_HEIGHT];
        Ok(())
    }
}
