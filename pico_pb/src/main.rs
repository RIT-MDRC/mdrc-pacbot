#![no_std]
#![no_main]

#[allow(dead_code)]
mod devices;
#[allow(dead_code)]
mod encoders;
mod logging;
mod motors;
mod network;
mod peripherals;

// todo https://github.com/adafruit/Adafruit_CircuitPython_seesaw/blob/main/adafruit_seesaw/seesaw.py https://crates.io/crates/adafruit-seesaw
use crate::encoders::{run_encoders, PioEncoder};
use crate::motors::{Motors, MOTORS_CHANNEL};
use crate::network::{initialize_network, Network, NETWORK_CHANNEL};
use crate::peripherals::{RobotPeripherals, PERIPHERALS_CHANNEL};
use core::ops::{Deref, DerefMut};
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::{network_task, RobotNetworkBehavior};
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::{RobotInterTaskMessage, RobotTaskMessenger};
use core_pb::messages::Task;
use core_pb::names::RobotName;
use core_pb::robot_definition::RobotDefinition;
use core_pb::util::CrossPlatformInstant;
use defmt::{debug, info, unwrap};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_rp::gpio::Pin;
use embassy_rp::i2c::{Async, I2c};
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::peripherals::{I2C1, PIO0, PIO1};
use embassy_rp::pio::Pio;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, interrupt};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use panic_probe as _;
use static_cell::StaticCell;

pub type PacbotI2cBus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;
pub type PacbotI2cDevice = I2cDevice<'static, NoopRawMutex, I2c<'static, I2C1, Async>>;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO1>;
    I2C1_IRQ => embassy_rp::i2c::InterruptHandler<I2C1>;
});

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR_HIGH.on_interrupt()
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello world!");

    let p = embassy_rp::init(Default::default());

    // Override bootloader watchdog
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(Duration::from_secs(8));

    // Initialize encoders
    let Pio {
        mut common,
        sm0,
        sm1,
        sm2,
        ..
    } = Pio::new(p.PIO1, Irqs);

    let encoder_a = PioEncoder::new(&mut common, sm0, p.PIN_4, p.PIN_5);
    let encoder_b = PioEncoder::new(&mut common, sm1, p.PIN_8, p.PIN_9);
    let encoder_c = PioEncoder::new(&mut common, sm2, p.PIN_12, p.PIN_13);

    // High-priority executor: SWI_IRQ_1, priority level 2
    interrupt::SWI_IRQ_1.set_priority(Priority::P2);
    let int_spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);
    unwrap!(int_spawner.spawn(run_encoders((encoder_a, encoder_b, encoder_c))));

    // Initialize network
    let mut network = initialize_network(
        spawner, p.PIN_23, p.PIN_25, p.PIO0, p.PIN_24, p.PIN_29, p.DMA_CH0, p.FLASH,
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
            (p.PIN_2, p.PIN_3, p.PIN_6, p.PIN_7, p.PIN_10, p.PIN_11),
            (p.PWM_SLICE1, p.PWM_SLICE3, p.PWM_SLICE5),
        )
    )));

    // xshut pins array
    let xshut = [
        p.PIN_14.degrade(),
        p.PIN_15.degrade(),
        p.PIN_16.degrade(),
        p.PIN_17.degrade(),
    ];

    // set up shared I2C
    static I2C_BUS: StaticCell<PacbotI2cBus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(embassy_rp::i2c::I2c::new_async(
        p.I2C1,
        p.PIN_27,
        p.PIN_26,
        Irqs,
        embassy_rp::i2c::Config::default(),
    )));
    unwrap!(spawner.spawn(do_i2c(name, RobotPeripherals::new(i2c_bus, xshut, spawner))));

    info!("Finished spawning tasks");

    loop {
        debug!("I'm alive!");
        watchdog.feed();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn do_wifi(network: Network) {
    unwrap!(network_task(network, Messenger(Task::Wifi)).await);
}

#[embassy_executor::task]
async fn do_motors(name: RobotName, motors: Motors<3>) {
    unwrap!(motors_task(name, motors, Messenger(Task::Motors)).await)
}

#[embassy_executor::task]
async fn do_i2c(name: RobotName, i2c: RobotPeripherals) {
    unwrap!(peripherals_task(name, i2c, Messenger(Task::Peripherals)).await)
}

pub struct Messenger(Task);

impl RobotTaskMessenger for Messenger {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        match to {
            Task::Wifi => NETWORK_CHANNEL.try_send(message),
            Task::Motors => MOTORS_CHANNEL.try_send(message),
            Task::Peripherals => PERIPHERALS_CHANNEL.try_send(message),
        }
        .is_ok()
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        match to {
            Task::Wifi => NETWORK_CHANNEL.send(message).await,
            Task::Motors => MOTORS_CHANNEL.send(message).await,
            Task::Peripherals => PERIPHERALS_CHANNEL.send(message).await,
        }
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        let channel = match self.0 {
            Task::Wifi => &NETWORK_CHANNEL,
            Task::Motors => &MOTORS_CHANNEL,
            Task::Peripherals => &PERIPHERALS_CHANNEL,
        };
        channel.receive().await
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: core::time::Duration,
    ) -> Option<RobotInterTaskMessage> {
        let channel = match self.0 {
            Task::Wifi => &NETWORK_CHANNEL,
            Task::Motors => &MOTORS_CHANNEL,
            Task::Peripherals => &PERIPHERALS_CHANNEL,
        };
        match select(channel.receive(), Timer::after(timeout.try_into().unwrap())).await {
            Either::First(msg) => Some(msg),
            Either::Second(_) => None,
        }
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
        Instant::elapsed(self).into()
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
