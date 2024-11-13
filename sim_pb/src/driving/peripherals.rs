use crate::driving::SimRobot;
use bevy::log::error;
use core_pb::constants::{ROBOT_DISPLAY_HEIGHT, ROBOT_DISPLAY_WIDTH};
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::messages::{ExtraImuData, RobotButton};
use core_pb::util::StdInstant;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::Pixel;
use std::sync::{Arc, RwLock};

pub struct SimPeripherals {
    robot: Arc<RwLock<SimRobot>>,
}

impl SimPeripherals {
    pub fn new(robot: Arc<RwLock<SimRobot>>) -> Self {
        Self { robot }
    }
}

#[derive(Debug)]
pub enum SimPeripheralsError {
    Unknown,
}

impl RobotPeripheralsBehavior for SimPeripherals {
    type Display = SimDisplay;
    type Instant = StdInstant;
    type Error = SimPeripheralsError;

    async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display) -> Result<(), SimPeripheralsError>,
    {
        let mut robot = self.robot.write().unwrap();
        if let Err(e) = draw(&mut robot.display) {
            error!("Error drawing: {e:?}")
        }
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

    async fn extra_imu_data(&mut self) -> Option<ExtraImuData> {
        None
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
