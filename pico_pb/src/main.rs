#![no_std]
#![no_main]

#[allow(dead_code)]
mod encoders;
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

use crate::encoders::{run_encoders, PioEncoder};
use crate::i2c::{RobotPeripherals, PERIPHERALS_CHANNEL};
use crate::motors::{Motors, MOTORS_CHANNEL};
use crate::network::{initialize_network, Network, NETWORK_CHANNEL};
use core::ops::{Deref, DerefMut};
use core_pb::driving::motors::motors_task;
#[allow(unused_imports)]
use core_pb::driving::network::{network_task, RobotNetworkBehavior};
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{info, RobotInterTaskMessage, Task};
use core_pb::names::RobotName;
use core_pb::robot_definition::RobotDefinition;
use core_pb::util::CrossPlatformInstant;
use defmt::unwrap;
use defmt_rtt as _;
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::peripherals::{I2C0, PIO0, PIO1};
use embassy_rp::pio::Pio;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, interrupt};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO1>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<I2C0>;
});

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR_HIGH.on_interrupt()
}

fn send_or_drop2(message: RobotInterTaskMessage, to: Task) -> bool {
    let result = match to {
        Task::Wifi => NETWORK_CHANNEL.try_send(message),
        Task::Motors => MOTORS_CHANNEL.try_send(message),
        Task::Peripherals => PERIPHERALS_CHANNEL.try_send(message),
    };
    match result {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn send_blocking2(message: RobotInterTaskMessage, to: Task) {
    match to {
        Task::Wifi => NETWORK_CHANNEL.send(message).await,
        Task::Motors => MOTORS_CHANNEL.send(message).await,
        Task::Peripherals => PERIPHERALS_CHANNEL.send(message).await,
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello world!");

    let p = embassy_rp::init(Default::default());

    // Override bootloader watchdog
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(Duration::from_secs(8));

    let Pio {
        mut common,
        sm0,
        sm1,
        sm2,
        ..
    } = Pio::new(p.PIO1, Irqs);

    let encoder_a = PioEncoder::new(&mut common, sm0, p.PIN_18, p.PIN_19);
    let encoder_b = PioEncoder::new(&mut common, sm1, p.PIN_20, p.PIN_21);
    let encoder_c = PioEncoder::new(&mut common, sm2, p.PIN_26, p.PIN_27);

    // High-priority executor: SWI_IRQ_1, priority level 2
    interrupt::SWI_IRQ_1.set_priority(Priority::P2);
    let int_spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);
    unwrap!(int_spawner.spawn(run_encoders((encoder_a, encoder_b, encoder_c))));

    let mut network = initialize_network(
        spawner.clone(),
        p.PIN_23,
        p.PIN_25,
        p.PIO0,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
        p.FLASH,
    )
    .await;

    let mac_address = network.mac_address().await;
    info!("mac_address {:?}", mac_address);

    let name = RobotName::from_mac_address(&mac_address).expect("Unrecognized mac address");
    info!("I am {}, mac address {:?}", name, mac_address);

    unwrap!(spawner.spawn(do_wifi(network)));
    unwrap!(spawner.spawn(do_motors(
        name,
        Motors::new(
            RobotDefinition::new(name),
            (p.PIN_6, p.PIN_7, p.PIN_8, p.PIN_9, p.PIN_14, p.PIN_15),
            (p.PWM_SLICE3, p.PWM_SLICE4, p.PWM_SLICE7),
        )
    )));
    unwrap!(spawner.spawn(do_i2c(RobotPeripherals::new(p.I2C0, p.PIN_17, p.PIN_16))));

    info!("Finished spawning tasks");

    loop {
        info!("I'm alive!");
        watchdog.feed();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn keep_watchdog_happy(mut watchdog: Watchdog) {
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
async fn do_motors(name: RobotName, motors: Motors<3>) {
    unwrap!(motors_task(name, motors).await)
}

#[embassy_executor::task]
async fn do_i2c(i2c: RobotPeripherals) {
    unwrap!(peripherals_task(i2c).await)
}

async fn receive_timeout(
    channel: &Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64>,
    timeout: core::time::Duration,
) -> Option<RobotInterTaskMessage> {
    match select(channel.receive(), Timer::after(timeout.try_into().unwrap())).await {
        Either::First(msg) => Some(msg),
        Either::Second(_) => None,
    }
}

#[derive(Copy, Clone)]
pub struct EmbassyInstant(Instant);

impl Deref for EmbassyInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EmbassyInstant {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CrossPlatformInstant for EmbassyInstant {
    fn elapsed(&self) -> core::time::Duration {
        Instant::elapsed(&self).into()
    }

    fn checked_duration_since(&self, other: Self) -> Option<core::time::Duration> {
        Instant::checked_duration_since(self, other.0).map(|x| x.into())
    }
}

impl Default for EmbassyInstant {
    fn default() -> Self {
        Self(Instant::now())
    }
}
