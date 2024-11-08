use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::i2c;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Timer};
use micromath::F32Ext;

pub type ImuError =
    bno08x_async::wrapper::WrapperError<bno08x_async::Error<I2cDeviceError<i2c::Error>, ()>>;

pub struct PacbotIMU {
    enabled: &'static AtomicBool,
    results: &'static Signal<ThreadModeRawMutex, Result<f32, PeripheralsError>>,

    sensor: bno08x_async::wrapper::BNO080<bno08x_async::interface::I2cInterface<PacbotI2cDevice>>,
    initialized: bool,
}

impl PacbotIMU {
    pub fn new(
        bus: &'static PacbotI2cBus,
        enabled: &'static AtomicBool,
        results: &'static Signal<ThreadModeRawMutex, Result<f32, PeripheralsError>>,
    ) -> Self {
        Self {
            enabled,
            results,

            initialized: false,
            sensor: bno08x_async::wrapper::BNO080::new_with_interface(
                bno08x_async::interface::I2cInterface::default(I2cDevice::new(bus)),
            ),
        }
    }

    pub async fn run_forever(mut self) -> ! {
        loop {
            match self.initialize().await {
                Ok(()) => {
                    self.sensor.handle_one_message(&mut Delay, 10).await;
                    self.results.signal(self.get_measurement().await);
                    Timer::after_millis(20).await;
                }
                Err(e) => {
                    self.results.signal(Err(e));
                    Timer::after_millis(300).await;
                }
            }
        }
    }

    async fn get_measurement(&mut self) -> Result<f32, PeripheralsError> {
        match self.sensor.rotation_quaternion() {
            Err(e) => {
                self.initialized = false;
                Err(PeripheralsError::ImuError(e))
            }
            Ok(quat) => {
                // convert quat to angle (yaw)
                // https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles
                let siny_cosp = 2.0 * (quat[3] * quat[2] + quat[0] * quat[1]);
                let cosy_cosp = 1.0 - 2.0 * (quat[1] * quat[1] + quat[2] * quat[2]);
                let yaw = f32::atan2(siny_cosp, cosy_cosp);
                defmt::debug!("IMU yaw reading: {}", yaw);
                Ok(yaw)
            }
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

        info!("Attempting to initialize bno08x IMU");

        // initialize sensor
        self.sensor
            .init(&mut Delay)
            .await
            .map_err(PeripheralsError::ImuInitErr)?;
        self.sensor
            .enable_rotation_vector(10)
            .await
            .map_err(PeripheralsError::ImuInitErr)?;

        self.initialized = true;
        Ok(())
    }
}
