use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice};
use core::sync::atomic::Ordering;
use defmt::info;
use display_interface::DisplayError;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Instant;
use portable_atomic::AtomicBool;
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use ssd1306::Ssd1306Async;

const DISPLAY_ADDRESS: u8 = 0x3d;

pub type PacbotDisplay = Ssd1306Async<
    I2CInterface<PacbotI2cDevice>,
    DisplaySize128x64,
    BufferedGraphicsModeAsync<DisplaySize128x64>,
>;

impl From<DisplayError> for PeripheralsError {
    fn from(value: DisplayError) -> Self {
        Self::DisplayError(value)
    }
}

pub struct PacbotDisplayWrapper {
    enabled: &'static AtomicBool,

    display: PacbotDisplay,
    initialized: Result<(), PeripheralsError>,
    last_init_time: Instant,
}

#[allow(dead_code)]
impl PacbotDisplayWrapper {
    pub fn new(bus: &'static PacbotI2cBus, enabled: &'static AtomicBool) -> Self {
        let i2c_device = I2cDevice::new(bus);
        let interface = I2CInterface::new(i2c_device, DISPLAY_ADDRESS, 0x40);
        let display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate180)
            .into_buffered_graphics_mode();

        Self {
            enabled,

            display,
            initialized: Err(PeripheralsError::Uninitialized),
            last_init_time: Instant::now(),
        }
    }

    pub fn status(&self) -> Result<(), PeripheralsError> {
        self.initialized.clone()
    }

    pub async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut PacbotDisplay) -> Result<(), DisplayError>,
    {
        if self.initialize().await.is_ok() {
            self.initialized = draw(&mut self.display).map_err(PeripheralsError::DisplayError);
        }
    }

    pub async fn flush(&mut self) {
        if self.initialize().await.is_ok() {
            self.initialized = self
                .display
                .flush()
                .await
                .map_err(PeripheralsError::DisplayError);
        }
    }

    async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the display is OK, or if initialization has been attempted recently
        if self.initialized.is_ok() || self.last_init_time.elapsed().as_millis() < 500 {
            return self.initialized.clone();
        }

        self.last_init_time = Instant::now();

        info!("Attempting to initialize ssd1306 display");

        async fn init(display: &mut PacbotDisplayWrapper) -> Result<(), DisplayError> {
            display.display.init().await?;
            display.display.set_display_on(true).await?;
            display.display.clear_buffer();
            display.display.flush().await?;

            Ok(())
        }

        self.initialized = init(self).await.map_err(PeripheralsError::DisplayError);

        self.initialized.clone()
    }
}
