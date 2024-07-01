#![no_std]
#![no_main]

mod i2c;
mod vl53l1x;
mod vl6180x;

// todo distance sensor https://crates.io/crates/vl53l1x
// todo https://github.com/adafruit/Adafruit_CircuitPython_seesaw/blob/main/adafruit_seesaw/seesaw.py https://crates.io/crates/adafruit-seesaw
// todo https://github.com/adafruit/Adafruit_SSD1306/blob/master/Adafruit_SSD1306.cpp#L992 https://crates.io/crates/ssd1306
// todo https://github.com/adafruit/Adafruit_CircuitPython_BNO055/blob/main/adafruit_bno055.py https://crates.io/crates/bno055

use core::future::{ready, Future};
// use core_pb::driving::RobotBehavior;
use cortex_m_rt::entry;
use embassy_executor::Spawner;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    // println!("Hello, world!");
    loop {}
}

pub struct Robot {}

// impl RobotBehavior for Robot {
//     fn spawn_task<F>(task: F)
//     where
//         F: FnOnce() -> dyn Future<Output = ()>,
//     {
//         todo!()
//     }
//
//     fn get_distance_sensor() -> impl Future<Output = ()> + Send {
//         ready(())
//     }
// }
