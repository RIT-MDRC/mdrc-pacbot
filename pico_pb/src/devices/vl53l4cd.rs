use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::gpio::Output;
use embassy_rp::i2c;
use embassy_rp::i2c::Async;
use embassy_rp::peripherals::I2C0;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Delay, Instant, Timer};
use embedded_hal_async::i2c::I2c;
use vl53l4cd::wait::Poll;
use vl53l4cd::{Status, Vl53l4cd};

type PacbotDistanceSensorType =
    Vl53l4cd<I2cDevice<'static, NoopRawMutex, i2c::I2c<'static, I2C0, Async>>, Delay, Poll>;

impl From<vl53l4cd::Error<I2cDeviceError<i2c::Error>>> for PeripheralsError {
    fn from(_value: vl53l4cd::Error<I2cDeviceError<i2c::Error>>) -> Self {
        Self::DistanceSensorError(None)
    }
}

impl From<I2cDeviceError<i2c::Error>> for PeripheralsError {
    fn from(_value: I2cDeviceError<i2c::Error>) -> Self {
        Self::DistanceSensorError(None)
    }
}

pub struct PacbotDistanceSensor {
    enabled: bool,

    index: usize,
    addr: u8,

    last_measurement: Result<Option<u16>, PeripheralsError>,
    has_update: bool,

    last_init_time: Instant,
    default_sensor: PacbotDistanceSensorType,
    i2c_device: PacbotI2cDevice,
    sensor: PacbotDistanceSensorType,

    xshut: Output<'static>,
}

impl PacbotDistanceSensor {
    pub fn new(
        bus: &'static PacbotI2cBus,
        mut xshut: Output<'static>,
        index: usize,
        addr: u8,
    ) -> Self {
        // set XSHUT low to turn the sensor off
        xshut.set_low();

        Self {
            enabled: true,

            index,
            addr,

            last_measurement: Err(PeripheralsError::Uninitialized),
            has_update: true,

            last_init_time: Instant::now(),
            default_sensor: Vl53l4cd::new(I2cDevice::new(bus), Delay, Poll),
            i2c_device: I2cDevice::new(bus),
            sensor: Vl53l4cd::with_addr(I2cDevice::new(bus), addr, Delay, Poll),

            xshut,
        }
    }

    #[allow(dead_code)]
    pub fn set_enabled(&mut self, new_enabled: bool) {
        self.enabled = new_enabled;
        if !new_enabled {
            self.xshut.set_low();
            self.last_measurement = Err(PeripheralsError::Disabled)
        }
    }

    pub fn get_result(&mut self) -> Result<Option<u16>, PeripheralsError> {
        self.has_update = false;
        self.last_measurement.clone()
    }

    pub async fn update(&mut self) -> bool {
        if self.initialize().await.is_ok() {
            if let Err(e) = self.fetch_measurement().await {
                self.has_update = true;
                self.last_measurement = Err(e)
            }
        }
        self.has_update
    }

    async fn fetch_measurement(&mut self) -> Result<(), PeripheralsError> {
        if self.sensor.has_measurement().await? {
            let measurement = self.sensor.read_measurement().await?;
            self.has_update = true;
            self.last_measurement = match measurement.status {
                Status::Valid => Ok(Some(measurement.distance)),
                Status::DistanceBelowDetectionThreshold => Ok(Some(0)),
                Status::SignalTooWeak => Ok(None),
                status => Err(PeripheralsError::DistanceSensorError(Some(status))),
            };
        }
        Ok(())
    }

    pub async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled {
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the display is OK, or if initialization has been attempted recently
        if self.last_measurement.is_ok() || self.last_init_time.elapsed().as_millis() < 500 {
            return self.last_measurement.clone().map(|_| ());
        }

        self.last_init_time = Instant::now();

        info!(
            "Attempting to initialize vl53l4cd distance sensor {}",
            self.index
        );

        self.last_measurement = {
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

            Ok(None)
        };

        self.last_measurement.clone().map(|_| ())
    }
}
