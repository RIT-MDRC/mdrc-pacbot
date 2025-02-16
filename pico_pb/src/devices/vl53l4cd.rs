use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use core::sync::atomic::Ordering;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::gpio::Output;
use embassy_rp::i2c;
use embassy_rp::i2c::Async;
use embassy_rp::peripherals::I2C1;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Timer};
use embedded_hal_async::i2c::I2c;
use portable_atomic::AtomicBool;
use vl53l4cd::wait::Poll;
use vl53l4cd::{Status, Vl53l4cd};
use embassy_sync::mutex::Mutex;

// TODO: do this a better way
static INIT_LOCK: Mutex<CriticalSectionRawMutex, i32> = Mutex::new(0);

type PacbotDistanceSensorType =
    Vl53l4cd<I2cDevice<'static, NoopRawMutex, i2c::I2c<'static, I2C1, Async>>, Delay, Poll>;

impl From<vl53l4cd::Error<I2cDeviceError<i2c::Error>>> for PeripheralsError {
    fn from(_value: vl53l4cd::Error<I2cDeviceError<i2c::Error>>) -> Self {
        Self::DistanceSensorError(None)
    }
}

pub struct PacbotDistanceSensor {
    enabled: &'static AtomicBool,
    results: &'static Signal<CriticalSectionRawMutex, Result<Option<f32>, PeripheralsError>>,

    index: usize,
    addr: u8,

    default_sensor: PacbotDistanceSensorType,
    i2c_device: PacbotI2cDevice,
    sensor: PacbotDistanceSensorType,
    xshut: Output<'static>,
    initialized: bool,
}

impl PacbotDistanceSensor {
    pub fn new(
        bus: &'static PacbotI2cBus,
        mut xshut: Output<'static>,
        index: usize,
        addr: u8,
        enabled: &'static AtomicBool,
        results: &'static Signal<CriticalSectionRawMutex, Result<Option<f32>, PeripheralsError>>,
    ) -> Self {
        // set XSHUT low to turn the sensor off
        xshut.set_low();

        Self {
            enabled,
            results,

            index,
            addr,

            default_sensor: Vl53l4cd::new(I2cDevice::new(bus), Delay, Poll),
            i2c_device: I2cDevice::new(bus),
            sensor: Vl53l4cd::with_addr(I2cDevice::new(bus), addr, Delay, Poll),
            xshut,
            initialized: false,
        }
    }

    pub async fn run_forever(mut self) -> ! {
        loop {
            if self.initialize().await.is_ok() {
                match self.fetch_measurement().await {
                    Err(PeripheralsError::AwaitingMeasurement) => Timer::after_millis(10).await,
                    Err(e) => {
                        // set XSHUT low to turn the sensor off
                        self.xshut.set_low();
                        self.results.signal(Err(e));
                        self.initialized = false;
                    }
                    Ok(m) => self.results.signal(Ok(m.map(|x| x as f32))),
                }
                Timer::after_millis(20).await;
            } else {
                // set XSHUT low to turn the sensor off
                self.xshut.set_low();
                self.results
                    .signal(Err(PeripheralsError::DistanceSensorError(None)));
                Timer::after_millis(300).await;
                self.initialized = false;
            }
        }
    }

    async fn fetch_measurement(&mut self) -> Result<Option<u16>, PeripheralsError> {
        if self.sensor.has_measurement().await? {
            let measurement2 = self.sensor.read_measurement().await?;
            let measurement = match measurement2.status {
                Status::Valid => Ok(Some(measurement2.distance)),
                Status::DistanceBelowDetectionThreshold => Ok(Some(0)),
                Status::SignalTooWeak => Ok(None),
                status => Err(PeripheralsError::DistanceSensorError(Some(status))),
            };
            self.sensor.clear_interrupt().await?;
            measurement
        } else {
            Err(PeripheralsError::AwaitingMeasurement)
        }
    }

    async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled.load(Ordering::Relaxed) {
            self.initialized = false;
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the sensor is OK
        if self.initialized {
            return Ok(());
        }

        // start critical section
        let _lock = INIT_LOCK.lock().await;

        info!(
            "Attempting to initialize vl53l4cd distance sensor {}",
            self.index
        );

        // set XSHUT high to turn the sensor on
        self.xshut.set_high();
        Timer::after_millis(300).await;

        // initialize sensor with default address
        self.default_sensor.init().await?;
        // change address
        // https://github.com/adafruit/Adafruit_CircuitPython_VL53L4CD/blob/main/adafruit_vl53l4cd.py
        self.i2c_device
            .write(vl53l4cd::PERIPHERAL_ADDR, &[0x00, 0x01, self.addr])
            .await?;
        Timer::after_millis(300).await;
        // initialize sensor with new address
        self.sensor.init().await?;
        self.sensor.start_ranging().await?;

        self.initialized = true;
        Ok(())
    }
}
