use crate::peripherals::{PeripheralsError, EXTRA_IMU_DATA_SIGNAL};
use crate::{PacbotI2cBus, PacbotI2cDevice};
use bno08x_async::constants::{
    SENSOR_REPORTID_ACCELEROMETER, SENSOR_REPORTID_GYROSCOPE, SENSOR_REPORTID_MAGNETIC_FIELD,
    SENSOR_REPORTID_ROTATION_VECTOR,
};
use core::sync::atomic::{AtomicBool, Ordering};
use core_pb::driving::EXTRA_OPTS_BOOL;
use core_pb::messages::ExtraImuData;
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
    extra_reports: bool,
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
            extra_reports: false,
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
                    self.results.signal(Ok(self.get_measurement().await));
                    EXTRA_IMU_DATA_SIGNAL.signal(ExtraImuData {
                        accel: self.sensor.accel,
                        gyro: self.sensor.gyro,
                        mag: self.sensor.mag,
                        rotation_vector: self.sensor.rotation_vector,
                    });
                    Timer::after_millis(20).await;
                }
                Err(e) => {
                    // self.results.signal(Err(e));
                    Timer::after_millis(300).await;
                }
            }
        }
    }

    async fn get_measurement(&mut self) -> f32 {
        let [i, j, k, real] = self.sensor.rotation_vector.0;
        // convert quat to angle (yaw)
        // https://github.com/sparkfun/SparkFun_BNO080_Arduino_Library/blob/main/src/SparkFun_BNO080_Arduino_Library.cpp#L493
        let mut dqw = real;
        let mut dqx = i;
        let mut dqy = j;
        let mut dqz = k;

        let norm = (dqw * dqw + dqx * dqx + dqy * dqy + dqz * dqz).sqrt();
        dqw = dqw / norm;
        dqx = dqx / norm;
        dqy = dqy / norm;
        dqz = dqz / norm;

        let ysq = dqy * dqy;

        let t3 = 2.0 * (dqw * dqz + dqx * dqy);
        let t4 = 1.0 - 2.0 * (ysq + dqz * dqz);
        let yaw = f32::atan2(t3, t4);
        yaw
    }

    async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled.load(Ordering::Relaxed) {
            self.initialized = false;
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the sensor is OK
        if self.initialized {
            // should we enable reports?
            let should_do_extra_reports = EXTRA_OPTS_BOOL[7].load(Ordering::Relaxed);
            if !self.extra_reports && should_do_extra_reports {
                self.sensor
                    .enable_report(SENSOR_REPORTID_ACCELEROMETER, 1000)
                    .await
                    .map_err(PeripheralsError::ImuInitErr)?;
                self.sensor
                    .enable_report(SENSOR_REPORTID_GYROSCOPE, 1000)
                    .await
                    .map_err(PeripheralsError::ImuInitErr)?;
                self.sensor
                    .enable_report(SENSOR_REPORTID_MAGNETIC_FIELD, 1000)
                    .await
                    .map_err(PeripheralsError::ImuInitErr)?;
                self.extra_reports = true;
                return Ok(());
            }
            // should we disable reports?
            else if self.extra_reports && !should_do_extra_reports {
                // reset sensor
                self.extra_reports = false;
                self.initialized = false;
            } else {
                return Ok(());
            }
        }

        info!("Attempting to initialize bno08x IMU");

        // initialize sensor
        self.sensor
            .init(&mut Delay)
            .await
            .map_err(PeripheralsError::ImuInitErr)?;
        self.sensor
            .enable_report(SENSOR_REPORTID_ROTATION_VECTOR, 20)
            .await
            .map_err(PeripheralsError::ImuInitErr)?;

        self.initialized = true;
        Ok(())
    }
}
