#![no_std]
#![no_main]

mod i2c;
mod motors;
mod network;
#[allow(dead_code)]
mod vl53l1x;
mod vl6180x;
// todo distance sensor https://crates.io/crates/vl53l1x
// todo https://github.com/adafruit/Adafruit_CircuitPython_seesaw/blob/main/adafruit_seesaw/seesaw.py https://crates.io/crates/adafruit-seesaw
// todo https://github.com/adafruit/Adafruit_SSD1306/blob/master/Adafruit_SSD1306.cpp#L992 https://crates.io/crates/ssd1306
// todo https://github.com/adafruit/Adafruit_CircuitPython_BNO055/blob/main/adafruit_bno055.py https://crates.io/crates/bno055

use crate::i2c::{RobotPeripherals, PERIPHERALS_CHANNEL};
use crate::motors::{Motors, MOTORS_CHANNEL};
use crate::network::{initialize_network, Network, NETWORK_CHANNEL};
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::network_task;
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{info, RobotInterTaskMessage, Task};
use core_pb::names::RobotName;
use defmt::unwrap;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{I2C0, PIO0};
use embassy_rp::watchdog::Watchdog;
use embassy_sync::channel::TrySendError;
use embassy_time::{Duration, Timer};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
});

async fn send(
    message: RobotInterTaskMessage,
    to: Task,
) -> Result<(), TrySendError<RobotInterTaskMessage>> {
    match to {
        Task::Wifi => NETWORK_CHANNEL.try_send(message),
        Task::Motors => MOTORS_CHANNEL.try_send(message),
        Task::Peripherals => PERIPHERALS_CHANNEL.try_send(message),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello world!");

    let p = embassy_rp::init(Default::default());

    // Override bootloader watchdog
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(Duration::from_secs(8));

    // let network = initialize_network(
    //     spawner.clone(),
    //     p.PIN_23,
    //     p.PIN_25,
    //     p.PIO0,
    //     p.PIN_24,
    //     p.PIN_29,
    //     p.DMA_CH0,
    //     p.FLASH,
    // )
    // .await;
    //
    // unwrap!(spawner.spawn(do_wifi(network)));
    // unwrap!(spawner.spawn(do_motors(Motors::new(
    //     (p.PIN_6, p.PIN_7, p.PIN_8, p.PIN_9, p.PIN_14, p.PIN_15),
    //     (p.PWM_SLICE3, p.PWM_SLICE4, p.PWM_SLICE7)
    // ))));
    // unwrap!(spawner.spawn(do_i2c(RobotPeripherals::new(p.I2C0, p.PIN_17, p.PIN_16))));

    loop {
        info!("I'm alive!");
        watchdog.feed();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn do_wifi(network: Network) {
    unwrap!(network_task(network).await);
}

#[embassy_executor::task]
async fn do_motors(motors: Motors<3>) {
    unwrap!(motors_task(RobotName::Pierre, motors).await)
}

#[embassy_executor::task]
async fn do_i2c(i2c: RobotPeripherals) {
    unwrap!(peripherals_task(i2c).await)
}
