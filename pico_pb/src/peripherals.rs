use crate::devices::bno08x::{ImuError, PacbotIMU};
use crate::devices::ssd1306::{PacbotDisplay, PacbotDisplayWrapper};
use crate::devices::vl53l4cd::PacbotDistanceSensor;
use crate::{EmbassyInstant, PacbotI2cBus};
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::RobotInterTaskMessage;
use core_pb::messages::RobotButton;
use defmt::Format;
use display_interface::DisplayError;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use vl53l4cd::Status;

/// numbr of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 4;
/// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] = [0x29, 0x31, 0x32, 0x33];

static PERIPHERALS_SIGNAL: Signal<
    ThreadModeRawMutex,
    (
        [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
        Result<f32, PeripheralsError>,
    ),
> = Signal::new();

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

pub struct RobotPeripherals {
    display: PacbotDisplayWrapper,

    distances: [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
    angle: Result<f32, PeripheralsError>,
    battery: Result<f32, PeripheralsError>,
}

impl RobotPeripherals {
    pub async fn new(bus: &'static PacbotI2cBus) -> Self {
        Self {
            display: PacbotDisplayWrapper::new(bus),

            distances: DIST_SENSOR_ADDRESSES.map(|_| Err(PeripheralsError::Uninitialized)),
            angle: Err(PeripheralsError::Uninitialized),
            battery: Err(PeripheralsError::Uninitialized),
        }
    }
}

#[derive(Clone, Debug, Format)]
pub enum PeripheralsError {
    Uninitialized,
    Disabled,
    DisplayError(DisplayError),
    DistanceSensorError(Option<Status>),
    ImuError(ImuError),
    #[allow(dead_code)]
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
        self.fetch_sensor_signal().await;
        self.angle.clone()
    }

    async fn distance_sensor(&mut self, index: usize) -> Result<Option<f32>, Self::Error> {
        self.fetch_sensor_signal().await;
        self.distances[index].clone()
    }

    async fn battery_level(&mut self) -> Result<f32, Self::Error> {
        self.fetch_sensor_signal().await;
        self.battery.clone()
    }

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)> {
        None
    }

    async fn read_joystick(&mut self) -> Option<(f32, f32)> {
        None
    }
}

impl RobotPeripherals {
    async fn fetch_sensor_signal(&mut self) {
        if let Some((dist, ang)) = PERIPHERALS_SIGNAL.try_take() {
            self.distances = dist;
            self.angle = ang;
        }
    }
}

#[embassy_executor::task]
pub async fn manage_pico_i2c(bus: &'static PacbotI2cBus, xshut: [AnyPin; NUM_DIST_SENSORS]) {
    let mut i = 0;
    let mut dist_sensors = xshut.map(|pin| {
        i += 1;
        PacbotDistanceSensor::new(
            bus,
            // initialize xshut pins with low output to disable sensors
            Output::new(pin, Level::Low),
            i,
            DIST_SENSOR_ADDRESSES[i - 1],
        )
    });
    let mut imu = PacbotIMU::new(bus);

    let mut distances = DIST_SENSOR_ADDRESSES.map(|_| Err(PeripheralsError::Uninitialized));
    let mut angle = Err(PeripheralsError::Uninitialized);

    loop {
        // fetch new values
        let mut changed = false;

        for (i, sensor) in dist_sensors.iter_mut().enumerate() {
            if sensor.update().await {
                distances[i] = sensor.get_result();
                changed = true;
            }
        }
        if imu.update().await {
            angle = imu.get_result();
            changed = true;
        }

        if changed {
            // convert errors to () and distances to f32
            PERIPHERALS_SIGNAL.signal((
                distances.clone().map(|d| d.map(|x| x.map(|x| x as f32))),
                angle.clone(),
            ))
        }
        Timer::after_millis(1).await;
    }
}
