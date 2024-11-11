use crate::devices::bno08x::{ImuError, PacbotIMU};
use crate::devices::ltc2943::Ltc2943;
use crate::devices::ssd1306::{PacbotDisplay, PacbotDisplayWrapper};
use crate::devices::vl53l4cd::PacbotDistanceSensor;
use crate::{EmbassyInstant, PacbotI2cBus};
use core::sync::atomic::AtomicBool;
use core_pb::constants::MM_PER_GU;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::RobotInterTaskMessage;
use core_pb::messages::RobotButton;
use defmt::Format;
use display_interface::DisplayError;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_executor::task;
use embassy_futures::join::join3;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::i2c;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use futures::future::join4;
use vl53l4cd::Status;

/// number of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 4;
/// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] = [0x31, 0x32, 0x33, 0x34];

static IMU_ENABLED: AtomicBool = AtomicBool::new(true);
static IMU_SIGNAL: Signal<ThreadModeRawMutex, Result<f32, PeripheralsError>> = Signal::new();

pub async fn run_imu(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) -> ! {
    PacbotIMU::new(bus, enabled, &IMU_SIGNAL)
        .run_forever()
        .await
}

static DIST_ENABLED: AtomicBool = AtomicBool::new(true);
static DIST_SIGNALS: [Signal<ThreadModeRawMutex, Result<Option<u16>, PeripheralsError>>;
    NUM_DIST_SENSORS] = [Signal::new(), Signal::new(), Signal::new(), Signal::new()];

pub async fn run_dist(
    enabled: &'static AtomicBool,
    bus: &'static PacbotI2cBus,
    index: usize,
    xshut: AnyPin,
) -> ! {
    PacbotDistanceSensor::new(
        bus,
        Output::new(xshut, Level::Low),
        index,
        DIST_SENSOR_ADDRESSES[index],
        enabled,
        &DIST_SIGNALS[index],
    )
    .run_forever()
    .await
}

static BATTERY_MONITOR_ENABLED: AtomicBool = AtomicBool::new(false);
static BATTERY_MONITOR_SIGNAL: Signal<ThreadModeRawMutex, Result<f32, PeripheralsError>> =
    Signal::new();

pub async fn run_battery_monitor(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) {
    Ltc2943::new(bus, enabled, &BATTERY_MONITOR_SIGNAL)
        .run_forever()
        .await
}

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

pub struct RobotPeripherals {
    display: PacbotDisplayWrapper,

    distances: [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
    angle: Result<f32, PeripheralsError>,
}

impl RobotPeripherals {
    pub fn new(bus: &'static PacbotI2cBus) -> Self {
        Self {
            display: PacbotDisplayWrapper::new(bus),

            distances: DIST_SENSOR_ADDRESSES.map(|_| Err(PeripheralsError::Uninitialized)),
            angle: Err(PeripheralsError::Uninitialized),
        }
    }
}

#[derive(Clone, Debug, Format)]
#[allow(dead_code)]
pub enum PeripheralsError {
    Uninitialized,
    Disabled,
    Timeout,
    AwaitingMeasurement,
    DisplayError(DisplayError),
    DistanceSensorError(Option<Status>),
    ImuInitErr(ImuError),
    ImuError(ImuError),
    I2cError,
    BatteryMonitorError,
    Unimplemented,
}

impl From<I2cDeviceError<i2c::Error>> for PeripheralsError {
    fn from(_value: I2cDeviceError<i2c::Error>) -> Self {
        Self::I2cError
    }
}

impl RobotPeripheralsBehavior for RobotPeripherals {
    type Display = PacbotDisplay;
    type Instant = EmbassyInstant;
    type Error = PeripheralsError;

    async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display) -> Result<(), DisplayError>,
    {
        self.display.draw_display(draw).await;
    }

    async fn flip_screen(&mut self) {
        self.display.flush().await;
    }

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error> {
        if let Some(rot) = IMU_SIGNAL.try_take() {
            self.angle = rot;
        }
        self.angle.clone()
    }

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error> {
        if let Some(dist) = DIST_SIGNALS[index].try_take() {
            self.distances[index] = dist.map(|x| {
                x.map(|y| {
                    // found via linear regression
                    let mut float_mm = y as f32 * 1.164826877 + -37.19636185;
                    float_mm = f32::max(float_mm, 0.0);
                    float_mm / MM_PER_GU
                })
            });
        }
        self.distances[index].clone()
    }

    async fn battery_level(&mut self) -> Result<f32, Self::Error> {
        Err(PeripheralsError::Unimplemented)
    }

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)> {
        None
    }

    async fn read_joystick(&mut self) -> Option<(f32, f32)> {
        None
    }
}

#[task]
pub async fn manage_pico_i2c(bus: &'static PacbotI2cBus, xshut: [AnyPin; NUM_DIST_SENSORS]) {
    let [a, b, c, d] = xshut;
    join3(
        run_imu(&IMU_ENABLED, bus),
        join4(
            run_dist(&DIST_ENABLED, bus, 0, a),
            run_dist(&DIST_ENABLED, bus, 1, b),
            run_dist(&DIST_ENABLED, bus, 2, c),
            run_dist(&DIST_ENABLED, bus, 3, d),
        ),
        run_battery_monitor(&BATTERY_MONITOR_ENABLED, bus),
    )
    .await;
}
