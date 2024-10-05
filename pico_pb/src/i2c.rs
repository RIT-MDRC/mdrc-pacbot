use crate::{receive_timeout, send_blocking2, send_or_drop2, EmbassyInstant, Irqs};
use core::time::Duration;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use defmt::Format;
use embassy_rp::i2c::{Async, SclPin, SdaPin};
use embassy_rp::peripherals::I2C0;
use embassy_rp::{i2c, Peripheral};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embedded_hal_async::i2c::I2c;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplayRotation, I2CInterface};
use ssd1306::size::DisplaySize128x64;
use ssd1306::Ssd1306;

pub static PERIPHERALS_CHANNEL: Channel<ThreadModeRawMutex, RobotInterTaskMessage, 64> =
    Channel::new();

pub struct RobotPeripherals {
    display: Ssd1306<
        I2CInterface<i2c::I2c<'static, I2C0, Async>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
}

impl RobotPeripherals {
    pub fn new(
        peri: impl Peripheral<P = I2C0> + 'static,
        scl: impl Peripheral<P = impl SclPin<I2C0>> + 'static,
        sda: impl Peripheral<P = impl SdaPin<I2C0>> + 'static,
    ) -> Self {
        let i2c = i2c::I2c::new_async(peri, scl, sda, Irqs, i2c::Config::default());

        let interface = I2CInterface::new(i2c, 0x3c, 0);
        let display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        // can't use i2c for anything else - see shared_bus crate

        // ...
        Self { display }
    }
}

#[derive(Debug, Format)]
pub enum I2cError {}

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
    type Display = Ssd1306<
        I2CInterface<i2c::I2c<'static, I2C0, Async>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >;
    type Instant = EmbassyInstant;
    type Error = ();

    fn draw_display<F>(&mut self, draw: F) -> Result<(), Self::Error>
    where
        F: FnOnce(&mut Self::Display) -> Result<(), display_interface::DisplayError>,
    {
        draw(&mut self.display).map_err(|_| ())
    }

    async fn flip_screen(&mut self) {
        let _ = self.display.flush();
    }

    async fn absolute_rotation(&mut self) -> Result<f32, Self::Error> {
        Err(())
    }

    async fn distance_sensor(&mut self, _index: usize) -> Result<Option<f32>, Self::Error> {
        Err(())
    }

    async fn battery_level(&mut self) -> Result<f32, Self::Error> {
        Err(())
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
