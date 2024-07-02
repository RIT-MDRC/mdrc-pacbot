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

use crate::i2c::{RobotI2c, I2C_CHANNEL};
use crate::motors::{Motors, MOTORS_CHANNEL};
use crate::network::{initialize_network, Network, NETWORK_CHANNEL};
use core_pb::driving::{
    i2c_task, motors_task, start_all_tasks, wifi_task, RobotBehavior, RobotInterTaskMessage, Task,
};
use defmt::unwrap;
use embassy_executor::{SpawnError, SpawnToken, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{I2C0, PIO0};
use embassy_sync::channel::TrySendError;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

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
        Task::I2c => I2C_CHANNEL.try_send(message),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let network = initialize_network(
        spawner.clone(),
        p.PIN_23,
        p.PIN_25,
        p.PIO0,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    )
    .await;

    start_all_tasks(Robot {
        spawner,
        wifi_task: Some(do_wifi(network)),
        motors_task: Some(do_motors(Motors {})),
        i2c_task: Some(do_i2c(RobotI2c {})),
    })
    .await
    .unwrap();
}

pub struct Robot<A, B, C> {
    spawner: Spawner,

    wifi_task: Option<SpawnToken<A>>,
    motors_task: Option<SpawnToken<B>>,
    i2c_task: Option<SpawnToken<C>>,
}

impl<A, B, C> RobotBehavior for Robot<A, B, C> {
    type SpawnError = SpawnError;

    fn spawn_wifi_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(self.wifi_task.take().unwrap())
    }

    fn spawn_motors_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(self.motors_task.take().unwrap())
    }

    fn spawn_i2c_task(&mut self) -> Result<(), Self::SpawnError> {
        self.spawner.spawn(self.i2c_task.take().unwrap())
    }
}

#[embassy_executor::task]
async fn do_wifi(network: Network) {
    unwrap!(wifi_task(network).await);
}

#[embassy_executor::task]
async fn do_motors(motors: Motors) {
    motors_task(motors).await
}

#[embassy_executor::task]
async fn do_i2c(i2c: RobotI2c) {
    i2c_task(i2c).await
}
