use crate::devices::bno08x::{ImuError, PacbotIMU};
use crate::devices::ssd1306::{PacbotDisplay, PacbotDisplayWrapper};
use crate::devices::vl53l4cd::PacbotDistanceSensor;
use crate::{EmbassyInstant, PacbotI2cBus};
use core::sync::atomic::AtomicBool;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::RobotInterTaskMessage;
use core_pb::messages::RobotButton;
use defmt::{unwrap, Format};
use display_interface::DisplayError;
use embassy_executor::{task, Spawner};
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use vl53l4cd::Status;

/// number of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 4;
/// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] = [0x31, 0x32, 0x33, 0x34];

static IMU_ENABLED: AtomicBool = AtomicBool::new(true);
static IMU_SIGNAL: Signal<ThreadModeRawMutex, Result<f32, PeripheralsError>> = Signal::new();

#[task]
pub async fn run_imu(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) -> ! {
    PacbotIMU::new(bus, enabled, &IMU_SIGNAL)
        .run_forever()
        .await
}

static DIST_ENABLED: AtomicBool = AtomicBool::new(true);
static DIST_SIGNALS: [Signal<ThreadModeRawMutex, Result<Option<u16>, PeripheralsError>>;
    NUM_DIST_SENSORS] = [Signal::new(), Signal::new(), Signal::new(), Signal::new()];

#[task]
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

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

pub struct RobotPeripherals {
    display: PacbotDisplayWrapper,

    distances: [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
    angle: Result<f32, PeripheralsError>,
}

impl RobotPeripherals {
    pub fn new(
        bus: &'static PacbotI2cBus,
        xshut: [AnyPin; NUM_DIST_SENSORS],
        spawner: Spawner,
    ) -> Self {
        unwrap!(spawner.spawn(run_imu(&IMU_ENABLED, bus)));

        for (i, xshut) in xshut.into_iter().enumerate() {
            unwrap!(spawner.spawn(run_dist(&DIST_ENABLED, bus, i, xshut)));
        }

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
    ImuError(ImuError),
    Unimplemented,
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
            self.distances[index] = dist.map(|x| x.map(|y| y as f32));
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
