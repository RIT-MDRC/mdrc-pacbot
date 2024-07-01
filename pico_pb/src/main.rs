#![no_std]
#![no_main]

mod i2c;
#[allow(dead_code)]
mod vl53l1x;
mod vl6180x;

// todo distance sensor https://crates.io/crates/vl53l1x
// todo https://github.com/adafruit/Adafruit_CircuitPython_seesaw/blob/main/adafruit_seesaw/seesaw.py https://crates.io/crates/adafruit-seesaw
// todo https://github.com/adafruit/Adafruit_SSD1306/blob/master/Adafruit_SSD1306.cpp#L992 https://crates.io/crates/ssd1306
// todo https://github.com/adafruit/Adafruit_CircuitPython_BNO055/blob/main/adafruit_bno055.py https://crates.io/crates/bno055

use core_pb::driving::{i2c_task, motors_task, start_robot, wifi_task, RobotBehavior};
use embassy_executor::{SpawnError, Spawner};
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let robot = Robot { spawner };
    start_robot(robot).await;
}

pub struct Robot {
    spawner: Spawner,
}

impl RobotBehavior for Robot {
    type SpawnError = SpawnError;

    fn spawn_wifi_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(do_wifi())
    }

    fn spawn_motors_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(do_motors())
    }

    fn spawn_i2c_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(do_i2c())
    }
}

#[embassy_executor::task]
async fn do_wifi() {
    wifi_task().await
}

#[embassy_executor::task]
async fn do_motors() {
    motors_task().await
}

#[embassy_executor::task]
async fn do_i2c() {
    i2c_task().await
}
