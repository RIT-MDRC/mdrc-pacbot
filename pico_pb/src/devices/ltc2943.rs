use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use portable_atomic::AtomicBool;

const ADDRESS: u8 = 0b1100100;

pub struct Ltc2943 {
    enabled: &'static AtomicBool,
    results: &'static Signal<CriticalSectionRawMutex, Result<f32, PeripheralsError>>,

    i2c_device: PacbotI2cDevice,
    initialized: bool,
}

impl Ltc2943 {
    pub fn new(
        bus: &'static PacbotI2cBus,
        enabled: &'static AtomicBool,
        results: &'static Signal<CriticalSectionRawMutex, Result<f32, PeripheralsError>>,
    ) -> Self {
        Self {
            enabled,
            results,

            i2c_device: I2cDevice::new(bus),
            initialized: false,
        }
    }

    pub async fn run_forever(mut self) -> ! {
        loop {
            match self.initialize().await {
                Ok(()) => {
                    self.results.signal(self.get_result().await);
                    Timer::after_secs(10).await;
                }
                Err(e) => {
                    self.initialized = false;
                    self.results.signal(Err(e));
                    Timer::after_millis(3000).await;
                }
            }
        }
    }

    async fn get_result(&mut self) -> Result<f32, PeripheralsError> {
        let mut regs = [0; 2];
        self.i2c_device.write_read(ADDRESS, &[8], &mut regs).await?;
        // from datasheet
        let reg_value = ((regs[0] as u16) << 8) | (regs[1] as u16);
        let voltage = 23.6 * (reg_value as f32 / 0xFFFF as f32);
        Ok(voltage)
    }

    async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        #[allow(clippy::unusual_byte_groupings)]
        self.i2c_device
            .write(ADDRESS, &[0x01, 0b10_111_00_0])
            .await
            .map_err(|_| PeripheralsError::I2cError)
    }
}
