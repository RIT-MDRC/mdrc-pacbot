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
use crate::motors::Motors;
use crate::network::{initialize_network, Network};
use crate::peripherals::{manage_pico_i2c, Peripherals};
use core::ops::{Deref, DerefMut};
use core_pb::driving::data::SharedRobotData;
use core_pb::driving::motors::motors_task;
use core_pb::driving::network::{network_task, RobotNetworkBehavior};
use core_pb::driving::peripherals::peripherals_task;
use core_pb::driving::RobotBehavior;
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
use once_cell::sync::OnceCell;
use panic_probe as _;
use static_cell::StaticCell;

pub type PacbotI2cBus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;
pub type PacbotI2cDevice = I2cDevice<'static, NoopRawMutex, I2c<'static, I2C1, Async>>;

static SHARED_DATA: OnceCell<SharedRobotData<PicoRobotBehavior>> = OnceCell::new();

struct PicoRobotBehavior;
type SharedPicoRobotData = SharedRobotData<PicoRobotBehavior>;
impl RobotBehavior for PicoRobotBehavior {
    type Instant = EmbassyInstant;

    type Motors = Motors<3>;
    type Network = Network;
    type Peripherals = Peripherals;
}

impl PicoRobotBehavior {
    fn get() -> &'static SharedRobotData<Self> {
        SHARED_DATA
            .get()
            .expect("RobotBehavior get() called before initialization")
    }
}

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

    // Initialize network
    let mut network = initialize_network(
        spawner, p.PIN_23, p.PIN_25, p.PIO0, p.PIN_24, p.PIN_29, p.DMA_CH0, p.FLASH,
    )
    .await;

    let mac_address = network.mac_address().await;
    info!("mac_address {:?}", mac_address);

    let name = RobotName::from_mac_address(&mac_address).expect("Unrecognized mac address");
    info!("I am {}, mac address {:?}", name, mac_address);

    // Set up core's shared data
    // It is important that this happens before any core tasks begin
    let shared_data = SHARED_DATA.get_or_init(|| SharedRobotData::new(name));

    // High-priority executor: SWI_IRQ_1, priority level 2
    interrupt::SWI_IRQ_1.set_priority(Priority::P2);
    let int_spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);
    unwrap!(int_spawner.spawn(run_encoders(shared_data, (encoder_a, encoder_b, encoder_c))));

    unwrap!(spawner.spawn(do_wifi(network)));
    unwrap!(spawner.spawn(do_motors(Motors::new(
        shared_data,
        (p.PIN_2, p.PIN_3, p.PIN_6, p.PIN_7, p.PIN_10, p.PIN_11),
        (p.PWM_SLICE1, p.PWM_SLICE3, p.PWM_SLICE5),
    ))));

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
    unwrap!(spawner.spawn(do_i2c(Peripherals::new(i2c_bus))));
    unwrap!(spawner.spawn(manage_pico_i2c(i2c_bus, xshut)));

    info!("Finished spawning tasks");

    loop {
        debug!("I'm alive!");
        watchdog.feed();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn do_wifi(network: Network) {
    network_task::<PicoRobotBehavior>(PicoRobotBehavior::get(), network).await;
}

#[embassy_executor::task]
async fn do_motors(motors: Motors<3>) {
    motors_task::<PicoRobotBehavior>(PicoRobotBehavior::get(), motors).await
}

#[embassy_executor::task]
async fn do_i2c(peripherals: Peripherals) {
    peripherals_task::<PicoRobotBehavior>(PicoRobotBehavior::get(), peripherals).await
}

#[derive(Copy, Clone)]
pub struct EmbassyInstant(Instant);

impl CrossPlatformInstant for EmbassyInstant {
    fn elapsed(&self) -> core::time::Duration {
        Instant::elapsed(&self.0).into()
    }

    fn checked_duration_since(&self, other: Self) -> Option<core::time::Duration> {
        Instant::checked_duration_since(&self.0, other.0).map(|x| x.into())
    }

    async fn sleep(duration: core::time::Duration) {
        Timer::after(duration.try_into().unwrap()).await
    }
}

impl Default for EmbassyInstant {
    fn default() -> Self {
        Self(Instant::now())
    }
}
