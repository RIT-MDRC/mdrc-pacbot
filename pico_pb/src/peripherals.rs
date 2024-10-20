use crate::{
    receive_timeout, send_blocking2, send_or_drop2, EmbassyInstant, PacbotI2cBus, PacbotI2cDevice,
};
use core::time::Duration;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use defmt::{info, Format};
use display_interface::DisplayError;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::i2c;
use embassy_rp::i2c::Async;
use embassy_rp::peripherals::I2C0;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Instant, Timer};
use embedded_hal_async::i2c::I2c;
use micromath::F32Ext;
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use ssd1306::Ssd1306Async;
use vl53l4cd::wait::Poll;
use vl53l4cd::{Status, Vl53l4cd};

/// numbr of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 4;
/// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] = [0x29, 0x31, 0x32, 0x33];

const DISPLAY_ADDRESS: u8 = 0x3c;

static PERIPHERALS_SIGNAL: Signal<
    ThreadModeRawMutex,
    (
        [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
        Result<f32, PeripheralsError>,
    ),
> = Signal::new();

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

type PacbotDisplay = Ssd1306Async<
    I2CInterface<PacbotI2cDevice>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

pub struct RobotPeripherals {
    display: PacbotDisplay,
    display_initialized: bool,

    distances: [Result<Option<f32>, PeripheralsError>; NUM_DIST_SENSORS],
    angle: Result<f32, PeripheralsError>,
    battery: Result<f32, PeripheralsError>,
}

impl RobotPeripherals {
    pub fn new(bus: &'static PacbotI2cBus) -> Self {
        let i2c_device = I2cDevice::new(bus);
        let interface = I2CInterface::new(i2c_device, DISPLAY_ADDRESS, 0);
        let display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        Self {
            display,
            display_initialized: false,

            distances: DIST_SENSOR_ADDRESSES.map(|_| Err(PeripheralsError::Uninitialized)),
            angle: Err(PeripheralsError::Uninitialized),
            battery: Err(PeripheralsError::Uninitialized),
        }
    }
}

#[derive(Clone, Debug, Format)]
pub enum PeripheralsError {
    Uninitialized,
    DisplayError(DisplayError),
    DistanceSensorError(Option<Status>),
    ImuError(ImuError),
    #[allow(dead_code)]
    Unimplemented,
}

impl From<DisplayError> for PeripheralsError {
    fn from(value: DisplayError) -> Self {
        Self::DisplayError(value)
    }
}

impl RobotTask for RobotPeripherals {
    fn send_or_drop(&mut self, message: RobotInterTaskMessage, to: Task) -> bool {
        send_or_drop2(message, to)
    }

    async fn send_blocking(&mut self, message: RobotInterTaskMessage, to: Task) {
        send_blocking2(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        PERIPHERALS_CHANNEL.receive().await
    }

    async fn receive_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> Option<RobotInterTaskMessage> {
        receive_timeout(&PERIPHERALS_CHANNEL, timeout).await
    }
}

impl RobotPeripheralsBehavior for RobotPeripherals {
    type Display = PacbotDisplay;
    type Instant = EmbassyInstant;
    type Error = PeripheralsError;

    async fn draw_display<F>(&mut self, draw: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut Self::Display) -> Result<(), DisplayError>,
    {
        if !self.display_initialized {
            self.display.init().await?;
            self.display_initialized = true;
        }
        match draw(&mut self.display) {
            Err(e) => {
                self.display_initialized = false;
                Err(e.into())
            }
            _ => Ok(()),
        }
    }

    async fn flip_screen(&mut self) -> Result<(), Self::Error> {
        match self.display.flush().await {
            Err(e) => {
                self.display_initialized = false;
                Err(e.into())
            }
            _ => Ok(()),
        }
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
}

impl RobotPeripherals {
    async fn fetch_sensor_signal(&mut self) {
        if let Some((dist, ang)) = PERIPHERALS_SIGNAL.try_take() {
            self.distances = dist;
            self.angle = ang;
        }
    }
}

type PacbotDistanceSensorType =
    Vl53l4cd<I2cDevice<'static, NoopRawMutex, i2c::I2c<'static, I2C0, Async>>, Delay, Poll>;
type DistanceSensorError = vl53l4cd::Error<I2cDeviceError<i2c::Error>>;

struct PacbotDistanceSensor {
    initialized: bool,
    index: usize,
    addr: u8,
    last_initialization_attempt: Instant,

    last_measurement: Result<Option<u16>, PeripheralsError>,

    default_sensor: PacbotDistanceSensorType,
    i2c_device: PacbotI2cDevice,
    sensor: PacbotDistanceSensorType,

    xshut: Output<'static>,
}

impl PacbotDistanceSensor {
    fn new(bus: &'static PacbotI2cBus, mut xshut: Output<'static>, index: usize, addr: u8) -> Self {
        // set XSHUT low to turn the sensor off
        xshut.set_low();

        Self {
            initialized: false,
            index,
            addr,
            last_initialization_attempt: Instant::now(),

            last_measurement: Err(PeripheralsError::Uninitialized),

            default_sensor: Vl53l4cd::new(I2cDevice::new(bus), Delay, Poll),
            i2c_device: I2cDevice::new(bus),
            sensor: Vl53l4cd::with_addr(I2cDevice::new(bus), addr, Delay, Poll),

            xshut,
        }
    }

    async fn update(&mut self) -> Option<Result<Option<u16>, PeripheralsError>> {
        match self._update().await {
            Ok(None) => None,
            Ok(Some(dist)) => {
                self.last_measurement = Ok(dist);
                Some(Ok(dist))
            }
            Err(e) => {
                self.last_measurement = Err(e.clone());
                Some(Err(e))
            }
        }
    }

    async fn _update(&mut self) -> Result<Option<Option<u16>>, PeripheralsError> {
        if !self.initialized {
            if self.last_initialization_attempt.elapsed() < embassy_time::Duration::from_millis(500)
            {
                return Ok(None);
            }
            self.initialize()
                .await
                .map_err(|_| PeripheralsError::DistanceSensorError(None))?;
        }
        if self
            .sensor
            .has_measurement()
            .await
            .map_err(|_| PeripheralsError::DistanceSensorError(None))?
        {
            let measurement = self
                .sensor
                .read_measurement()
                .await
                .map_err(|_| PeripheralsError::DistanceSensorError(None))?;
            match measurement.status {
                Status::Valid => Ok(Some(Some(measurement.distance))),
                Status::DistanceBelowDetectionThreshold => Ok(Some(Some(0))),
                Status::SignalTooWeak => Ok(Some(None)),
                status => Err(PeripheralsError::DistanceSensorError(Some(status))),
            }
        } else {
            Ok(None)
        }
    }

    async fn initialize(&mut self) -> Result<(), DistanceSensorError> {
        self.last_initialization_attempt = Instant::now();

        info!("Attempting to initialize distance sensor {}", self.index);

        // set XSHUT high to turn the sensor on
        self.xshut.set_high();
        Timer::after_millis(50).await;

        // initialize sensor with default address
        self.default_sensor.init().await?;
        // change address
        // https://github.com/adafruit/Adafruit_CircuitPython_VL53L4CD/blob/main/adafruit_vl53l4cd.py
        self.i2c_device
            .write(vl53l4cd::PERIPHERAL_ADDR, &[0x0001, self.addr])
            .await?;
        Timer::after_millis(50).await;
        // initialize sensor with new address
        self.sensor.init().await?;
        self.sensor.start_ranging().await?;

        self.initialized = true;
        Ok(())
    }
}

struct PacbotIMU {
    initialized: bool,
    last_initialization_attempt: Instant,

    sensor: bno08x_async::wrapper::BNO080<bno08x_async::interface::I2cInterface<PacbotI2cDevice>>,
    delay_source: Delay,
}

type ImuError =
    bno08x_async::wrapper::WrapperError<bno08x_async::Error<I2cDeviceError<i2c::Error>, ()>>;

impl PacbotIMU {
    fn new(bus: &'static PacbotI2cBus) -> Self {
        Self {
            initialized: false,
            last_initialization_attempt: Instant::now(),

            sensor: bno08x_async::wrapper::BNO080::new_with_interface(
                bno08x_async::interface::I2cInterface::default(I2cDevice::new(bus)),
            ),
            delay_source: Delay,
        }
    }

    async fn update(&mut self) -> Option<Result<f32, PeripheralsError>> {
        if !self.initialized {
            if self.last_initialization_attempt.elapsed() < embassy_time::Duration::from_millis(500)
            {
                return None;
            }
            if let Err(e) = self.initialize().await {
                return Some(Err(PeripheralsError::ImuError(e)));
            }
        }
        let _ = self
            .sensor
            .handle_one_message(&mut self.delay_source, 10)
            .await
            > 0;

        match self.sensor.rotation_quaternion() {
            Err(e) => {
                self.initialized = false;
                Some(Err(PeripheralsError::ImuError(e)))
            }
            Ok(quat) => {
                // convert quat to angle (yaw)
                // https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles
                let siny_cosp = 2.0 * (quat[3] * quat[2] + quat[0] * quat[1]);
                let cosy_cosp = 1.0 - 2.0 * (quat[1] * quat[1] + quat[2] * quat[2]);
                let yaw = f32::atan2(siny_cosp, cosy_cosp);
                defmt::debug!("IMU yaw reading: {}", yaw);
                Some(Ok(yaw))
            }
        }
    }

    async fn initialize(&mut self) -> Result<(), ImuError> {
        self.last_initialization_attempt = Instant::now();

        info!("Attempting to initialize IMU");

        // initialize sensor
        self.sensor.init(&mut self.delay_source).await?;
        self.sensor.enable_rotation_vector(10).await?;

        self.initialized = true;
        Ok(())
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
            if let Some(updated_value) = sensor.update().await {
                distances[i] = updated_value;
                changed = true;
            }
        }
        if let Some(updated_value) = imu.update().await {
            angle = updated_value;
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

pub async fn write_u8<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u8,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            data,
        ],
    )
    .await
}

pub async fn write_u16<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u16,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            (data >> 8) as u8,
            (data & 0xFF) as u8,
        ],
    )
    .await
}

pub async fn write_u32<T: I2c>(
    address: u8,
    i2c: &mut T,
    location: u16,
    data: u32,
) -> Result<(), T::Error> {
    i2c.write(
        address,
        &[
            ((location >> 8) & 0xFF) as u8,
            (location & 0xFF) as u8,
            ((data >> 24) & 0xFF) as u8,
            ((data >> 16) & 0xFF) as u8,
            ((data >> 8) & 0xFF) as u8,
            (data & 0xFF) as u8,
        ],
    )
    .await
}

pub async fn read_u8<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u8, T::Error> {
    let mut buf = [0];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(buf[0])
}

pub async fn read_u16<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u16, T::Error> {
    let mut buf = [0; 2];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(u16::from_be_bytes([buf[0], buf[1]]))
}

pub async fn read_u32<T: I2c>(address: u8, i2c: &mut T, location: u16) -> Result<u32, T::Error> {
    let mut buf = [0; 4];
    i2c.write_read(
        address,
        &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
        &mut buf,
    )
    .await?;
    Ok(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
}
