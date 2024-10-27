use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::i2c;
use embassy_time::{Delay, Instant};
use micromath::F32Ext;

pub type ImuError =
    bno08x_async::wrapper::WrapperError<bno08x_async::Error<I2cDeviceError<i2c::Error>, ()>>;

impl From<ImuError> for PeripheralsError {
    fn from(value: ImuError) -> Self {
        Self::ImuError(value)
    }
}

pub struct PacbotIMU {
    enabled: bool,

    last_measurement: Result<f32, PeripheralsError>,
    has_update: bool,

    initialized: bool,
    last_init_time: Instant,

    sensor: bno08x_async::wrapper::BNO080<bno08x_async::interface::I2cInterface<PacbotI2cDevice>>,
}

impl PacbotIMU {
    pub fn new(bus: &'static PacbotI2cBus) -> Self {
        Self {
            enabled: true,

            last_measurement: Err(PeripheralsError::Uninitialized),
            has_update: true,

            initialized: false,
            last_init_time: Instant::now(),

            sensor: bno08x_async::wrapper::BNO080::new_with_interface(
                bno08x_async::interface::I2cInterface::default(I2cDevice::new(bus)),
            ),
        }
    }

    #[allow(dead_code)]
    pub fn set_enabled(&mut self, new_enabled: bool) {
        self.enabled = new_enabled;
        if !self.enabled {
            self.initialized = false;
        }
    }

    pub fn get_result(&mut self) -> Result<f32, PeripheralsError> {
        self.has_update = false;
        self.last_measurement.clone()
    }

    pub async fn update(&mut self) -> bool {
        if self.initialize().await.is_ok() {
            self.sensor.handle_one_message(&mut Delay, 10).await;

            self.has_update = true;
            self.last_measurement = match self.sensor.rotation_quaternion() {
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
        self.has_update
    }

    async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled {
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the display is OK, or if initialization has been attempted recently
        if self.last_measurement.is_ok() || self.last_init_time.elapsed().as_millis() < 500 {
            return self.last_measurement.clone().map(|_| ());
        }

        self.last_init_time = Instant::now();

        info!("Attempting to initialize bno08x IMU");

        async fn init(sensor: &mut PacbotIMU) -> Result<f32, PeripheralsError> {
            sensor.sensor.init(&mut Delay).await?;
            sensor.sensor.enable_rotation_vector(10).await?;

            Ok(0.0)
        }

        // initialize sensor
        self.last_measurement = init(self).await;

        self.last_measurement.clone().map(|_| ())
    }
}
