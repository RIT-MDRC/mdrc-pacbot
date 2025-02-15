use crate::peripherals::PeripheralsError;
use crate::{PacbotI2cBus, PacbotI2cDevice, PicoRobotBehavior};
use core::sync::atomic::Ordering;
use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_rp::i2c;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use portable_atomic::AtomicBool;

const STATUS_BASE: u8 = 0x00;

const STATUS_HW_ID: u8 = 0x01;
const STATUS_VERSION: u8 = 0x02;
const STATUS_OPTIONS: u8 = 0x03;
const STATUS_TEMP: u8 = 0x04;
const STATUS_SWRST: u8 = 0x7F;

const GPIO_BASE: u8 = 0x01;

const GPIO_DIRSET_BULK: u8 = 0x02;
const GPIO_DIRCLR_BULK: u8 = 0x03;
const GPIO_BULK: u8 = 0x04;
const GPIO_BULK_SET: u8 = 0x05;
const GPIO_BULK_CLR: u8 = 0x06;
const GPIO_BULK_TOGGLE: u8 = 0x07;
const GPIO_INTENSET: u8 = 0x08;
const GPIO_INTENCLR: u8 = 0x09;
const GPIO_INTFLAG: u8 = 0x0A;
const GPIO_PULLENSET: u8 = 0x0B;
const GPIO_PULLENCLR: u8 = 0x0C;

const I2C_ADDRESS: u8 = 0x50;

const BUTTON_X: u8 = 6;
const BUTTON_Y: u8 = 2;
const BUTTON_A: u8 = 5;
const BUTTON_B: u8 = 1;
const BUTTON_SELECT: u8 = 0;
const BUTTON_START: u8 = 16;

const BUTTON_MASK: u32 = (1 << BUTTON_X)
    | (1 << BUTTON_Y)
    | (1 << BUTTON_A)
    | (1 << BUTTON_B)
    | (1 << BUTTON_SELECT)
    | (1 << BUTTON_START);

type GamepadError = I2cDeviceError<i2c::Error>;

pub struct SeesawGamepadQt {
    enabled: &'static AtomicBool,

    is_down_x_y_a_b_select_start: [bool; 6],

    i2c: PacbotI2cDevice,
    initialized: bool,
}

impl SeesawGamepadQt {
    pub fn new(bus: &'static PacbotI2cBus, enabled: &'static AtomicBool) -> Self {
        Self {
            enabled,

            is_down_x_y_a_b_select_start: [false; 6],

            i2c: I2cDevice::new(bus),
            initialized: false,
        }
    }

    pub async fn run_forever(mut self) -> ! {
        loop {
            match self.initialize().await {
                Ok(()) => match self.read_inputs().await {
                    Ok(()) => {
                        Timer::after_millis(50).await;
                    }
                    Err(_) => {
                        self.initialized = false;
                        Timer::after_millis(1000).await;
                    }
                },
                Err(_) => {
                    self.initialized = false;
                    Timer::after_millis(1000).await;
                }
            }
        }
    }

    pub async fn initialize(&mut self) -> Result<(), PeripheralsError> {
        // do nothing if disabled
        if !self.enabled.load(Ordering::Relaxed) {
            self.initialized = false;
            return Err(PeripheralsError::Disabled);
        }

        // do nothing if the sensor is OK
        if self.initialized {
            return Ok(());
        }

        info!("Attempting to initialize seesaw gamepad QT");

        self.reset().await?;
        let mut chip_id = [0];
        self.read(STATUS_BASE, STATUS_HW_ID, &mut chip_id).await?;
        if chip_id[0] != 0x87 {
            error!("gamepad QT had wrong HW ID");
            return Err(PeripheralsError::I2cError);
        }

        self.write32(GPIO_BASE, GPIO_DIRCLR_BULK, BUTTON_MASK)
            .await?;
        self.write32(GPIO_BASE, GPIO_PULLENSET, BUTTON_MASK).await?;
        self.write32(GPIO_BASE, GPIO_BULK_SET, BUTTON_MASK).await?;

        self.initialized = true;
        Ok(())
    }

    pub async fn read_inputs(&mut self) -> Result<(), PeripheralsError> {
        let mut buf = [0; 4];
        self.read(GPIO_BASE, GPIO_BULK, &mut buf).await?;
        let buttons = u32::from_be_bytes(buf);
        [
            BUTTON_X,
            BUTTON_Y,
            BUTTON_A,
            BUTTON_B,
            BUTTON_SELECT,
            BUTTON_START,
        ]
        .iter()
        .enumerate()
        .for_each(|(i, x)| {
            let new = buttons & (1 << *x) != 0;
            let old = self.is_down_x_y_a_b_select_start[i];
            if old && !new {
                info!("Button {} pressed", i);
                PicoRobotBehavior::get().buttons[i].store(true, Ordering::Relaxed)
            }
            self.is_down_x_y_a_b_select_start[i] = new;
        });
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<(), GamepadError> {
        self.initialized = false;
        self.write8(STATUS_BASE, STATUS_SWRST, 0xFF).await?;
        Timer::after_millis(500).await;
        Ok(())
    }

    pub async fn write8(&mut self, reg_base: u8, reg: u8, value: u8) -> Result<(), GamepadError> {
        self.i2c.write(I2C_ADDRESS, &[reg_base, reg, value]).await
    }

    pub async fn write32(&mut self, reg_base: u8, reg: u8, value: u32) -> Result<(), GamepadError> {
        let bytes = value.to_be_bytes();
        self.i2c
            .write(
                I2C_ADDRESS,
                &[reg_base, reg, bytes[0], bytes[1], bytes[2], bytes[3]],
            )
            .await
    }

    pub async fn read(
        &mut self,
        reg_base: u8,
        reg: u8,
        result: &mut [u8],
    ) -> Result<(), GamepadError> {
        self.i2c
            .write_read(I2C_ADDRESS, &[reg_base, reg], result)
            .await
    }
}
