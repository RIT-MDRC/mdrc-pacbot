//! Based on https://github.com/adafruit/Adafruit_CircuitPython_VL6180X

use defmt::Format;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Debug, Format)]
#[repr(u8)]
pub enum RangeStatusError {
    /// Valid measurement
    None = 0,
    /// System error detected (can only happen on power on). No measurement possible
    SysErr1 = 1,
    /// System error detected (can only happen on power on). No measurement possible
    SysErr5 = 5,
    /// ECE check failed
    EceFail = 6,
    /// System did not converge before the specified max. convergence time limit
    NoConverge = 7,
    /// Ignore threshold check failed
    RangeIgnore = 8,
    /// Ambient conditions too high. Measurement not valid
    Snr = 11,
    /// Range value < 0
    RawUFlow = 12,
    /// Range value out of range
    RowOFlow = 13,
    /// Range value < 0
    RangeUFlow = 14,
    /// Range value out of range
    RangeOFlow = 15,
    /// Any other number
    UnknownError = 16,
}

impl RangeStatusError {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::None,
            1 => Self::SysErr1,
            5 => Self::SysErr5,
            6 => Self::EceFail,
            7 => Self::NoConverge,
            8 => Self::RangeIgnore,
            11 => Self::Snr,
            12 => Self::RawUFlow,
            13 => Self::RowOFlow,
            14 => Self::RangeUFlow,
            15 => Self::RangeOFlow,
            _ => Self::UnknownError,
        }
    }
}

const REG_IDENTIFICATION_MODEL_ID: u16 = 0x000;

const REG_SYSTEM_HISTORY_CTRL: u16 = 0x012;
// const REG_SYSTEM_INTERRUPT_CONFIG: u16 = 0x014;
const REG_SYSTEM_INTERRUPT_CLEAR: u16 = 0x015;
const REG_SYSTEM_FRESH_OUT_OF_RESET: u16 = 0x016;

const REG_SYSRANGE_START: u16 = 0x018;
const REG_SYSRANGE_INTERMEASUREMENT_PERIOD: u16 = 0x01B;
const REG_SYSRANGE_PART_TO_PART_RANGE_OFFSET: u16 = 0x024;

// const REG_SYSALS_ANALOGUE_GAIN: u16 = 0x03F;
// const REG_SYSALS_INTEGRATION_PERIOD_HI: u16 = 0x040;
// const REG_SYSALS_INTEGRATION_PERIOD_LO: u16 = 0x041;

const REG_RESULT_RANGE_STATUS: u16 = 0x04D;
const REG_RESULT_INTERRUPT_STATUS_GPIO: u16 = 0x04F;
const REG_RESULT_HISTORY_BUFFER_0: u16 = 0x052;
const REG_RESULT_RANGE_VAL: u16 = 0x062;

const REG_I2C_DEVICE_SLAVE_ADDRESS: u16 = 0x212;

/// Manages a VL6180X distance sensor, whether initialized or not
pub struct VL6180X {
    addr: u8,
    offset: u8,
}

#[allow(dead_code)]
impl VL6180X {
    /// Creates a new VL6180x and initializes it
    pub async fn new<T: I2c>(i2c: &mut T, addr: u8, offset: u8) -> Result<Self, T::Error> {
        let mut s = Self { addr, offset };
        s.try_initialize(i2c).await?;
        Ok(s)
    }

    /// Sets this device's address
    pub async fn set_address<T: I2c>(&mut self, i2c: &mut T, addr: u8) -> Result<(), T::Error> {
        let result = self.write_8(i2c, REG_I2C_DEVICE_SLAVE_ADDRESS, addr).await;
        self.addr = addr;
        result
    }

    /// Read the range of an object in front of sensor and return it in mm.
    ///
    /// Warning: this could block for a long time
    pub async fn await_range<T: I2c>(&self, i2c: &mut T) -> Result<u8, T::Error> {
        if self.continuous_mode_enabled(i2c).await? {
            self.read_range_continuous(i2c).await
        } else {
            self.read_range_single(i2c).await
        }
    }

    /// Read the latest range data from history.
    ///
    /// To do so, you don't have to wait for a complete measurement.
    pub async fn range_from_history<T: I2c>(&self, i2c: &mut T) -> Result<u8, T::Error> {
        self.read_8(i2c, REG_RESULT_HISTORY_BUFFER_0).await
    }

    /// Start continuous range mode
    ///
    /// period = 0 means 10ms intervals. Then + 1 adds 10ms, so period = 2 means 30ms intervals.
    pub async fn start_range_continuous<T: I2c>(
        &self,
        i2c: &mut T,
        period: u8,
    ) -> Result<(), T::Error> {
        // Set range between measurements
        self.write_8(i2c, REG_SYSRANGE_INTERMEASUREMENT_PERIOD, period)
            .await?;

        // Start continuous range measurement
        self.write_8(i2c, REG_SYSRANGE_START, 0x03).await
    }

    /// Stop continuous range mode
    pub async fn stop_range_continuous<T: I2c>(&self, i2c: &mut T) -> Result<(), T::Error> {
        if self.continuous_mode_enabled(i2c).await? {
            self.write_8(i2c, REG_SYSRANGE_START, 0x01).await?;
        }
        Ok(())
    }

    /// Checks if continuous mode is enabled
    pub async fn continuous_mode_enabled<T: I2c>(&self, i2c: &mut T) -> Result<bool, T::Error> {
        let x = self.read_8(i2c, REG_SYSRANGE_START).await?;
        Ok(x > 1)
    }

    /// Get the current sensor offset
    pub fn get_offset(&self) -> u8 {
        self.offset
    }

    /// Set the current sensor offset
    pub async fn set_offset<T: I2c>(&mut self, i2c: &mut T, offset: u8) -> Result<(), T::Error> {
        self.offset = offset;

        self.write_8(i2c, REG_SYSRANGE_PART_TO_PART_RANGE_OFFSET, offset)
            .await
    }

    /// Retrieve the status/error from a previous range read.
    pub async fn range_status<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<(u8, RangeStatusError), T::Error> {
        let x = self.read_8(i2c, REG_RESULT_RANGE_STATUS).await?;
        Ok((x, RangeStatusError::from_u8(x)))
    }

    async fn try_initialize<T: I2c>(&mut self, i2c: &mut T) -> Result<bool, T::Error> {
        if self.read_8(i2c, REG_IDENTIFICATION_MODEL_ID).await? != 0xB4 {
            // Device not found
            return Ok(false);
        }

        self.load_settings(i2c).await?;
        self.write_8(i2c, REG_SYSTEM_FRESH_OUT_OF_RESET, 0x00)
            .await?;

        // Reset a sensor that crashed while in continuous mode
        if self.continuous_mode_enabled(i2c).await? {
            // Stop continuous range mode. It is advised to wait for about 0.3s
            // afterward to avoid issues with the interrupt flags
            self.write_8(i2c, REG_SYSRANGE_START, 0x01).await?;
            Timer::after(Duration::from_millis(300)).await;
        }

        // Activate history buffer for range measurement
        self.write_8(i2c, REG_SYSTEM_HISTORY_CTRL, 0x01).await?;

        // Reset offset
        self.set_offset(i2c, self.offset).await?;

        Ok(true)
    }

    /// Read the range when in single-shot mode
    ///
    /// Warning: this could block for a long time
    async fn read_range_single<T: I2c>(&self, i2c: &mut T) -> Result<u8, T::Error> {
        while (self.read_8(i2c, REG_RESULT_RANGE_STATUS).await? & 0x01) == 0 {
            Timer::after(Duration::from_millis(1)).await;
        }
        self.write_8(i2c, REG_SYSRANGE_START, 0x01).await?;
        self.read_range_continuous(i2c).await
    }

    /// Read the range when in continuous mode
    ///
    /// Warning: this could block for a long time
    async fn read_range_continuous<T: I2c>(&self, i2c: &mut T) -> Result<u8, T::Error> {
        // Poll until bit 2 is set
        while (self.read_8(i2c, REG_RESULT_INTERRUPT_STATUS_GPIO).await? & 0x04) == 0 {
            Timer::after(Duration::from_millis(1)).await;
        }

        // Read range in mm
        let range = self.read_8(i2c, REG_RESULT_RANGE_VAL).await?;

        // Clear interrupt
        self.write_8(i2c, REG_SYSTEM_INTERRUPT_CLEAR, 0x07).await?;

        Ok(range)
    }

    async fn load_settings<T: I2c>(&self, i2c: &mut T) -> Result<(), T::Error> {
        // private settings from page 24 of app note
        self.write_8(i2c, 0x0207, 0x01).await?;
        self.write_8(i2c, 0x0208, 0x01).await?;
        self.write_8(i2c, 0x0096, 0x00).await?;
        self.write_8(i2c, 0x0097, 0xFD).await?;
        self.write_8(i2c, 0x00E3, 0x00).await?;
        self.write_8(i2c, 0x00E4, 0x04).await?;
        self.write_8(i2c, 0x00E5, 0x02).await?;
        self.write_8(i2c, 0x00E6, 0x01).await?;
        self.write_8(i2c, 0x00E7, 0x03).await?;
        self.write_8(i2c, 0x00F5, 0x02).await?;
        self.write_8(i2c, 0x00D9, 0x05).await?;
        self.write_8(i2c, 0x00DB, 0xCE).await?;
        self.write_8(i2c, 0x00DC, 0x03).await?;
        self.write_8(i2c, 0x00DD, 0xF8).await?;
        self.write_8(i2c, 0x009F, 0x00).await?;
        self.write_8(i2c, 0x00A3, 0x3C).await?;
        self.write_8(i2c, 0x00B7, 0x00).await?;
        self.write_8(i2c, 0x00BB, 0x3C).await?;
        self.write_8(i2c, 0x00B2, 0x09).await?;
        self.write_8(i2c, 0x00CA, 0x09).await?;
        self.write_8(i2c, 0x0198, 0x01).await?;
        self.write_8(i2c, 0x01B0, 0x17).await?;
        self.write_8(i2c, 0x01AD, 0x00).await?;
        self.write_8(i2c, 0x00FF, 0x05).await?;
        self.write_8(i2c, 0x0100, 0x05).await?;
        self.write_8(i2c, 0x0199, 0x05).await?;
        self.write_8(i2c, 0x01A6, 0x1B).await?;
        self.write_8(i2c, 0x01AC, 0x3E).await?;
        self.write_8(i2c, 0x01A7, 0x1F).await?;
        self.write_8(i2c, 0x0030, 0x00).await?;
        // Recommended : Public registers - See data sheet for more detail
        // Enables polling for 'New Sample ready'  when measurement completes
        self.write_8(i2c, 0x0011, 0x10).await?;
        // Set the averaging sample period (compromise between lower noise and increased execution time)
        self.write_8(i2c, 0x010A, 0x30).await?;
        // Sets the light and dark gain (upper nibble). Dark gain should not be changed.
        self.write_8(i2c, 0x003F, 0x46).await?;
        // Sets the # of range measurements after which auto calibration of system is performed
        self.write_8(i2c, 0x0031, 0xFF).await?;
        // Set ALS integration time to 100ms
        self.write_8(i2c, 0x0040, 0x63).await?;
        // Perform a single temperature calibration of the ranging sensor
        self.write_8(i2c, 0x002E, 0x01).await?;

        // Optional: Public registers - See data sheet for more detail
        // Set default ranging inter-measurement period to 100ms
        self.write_8(i2c, 0x001B, 0x09).await?;
        // Set default ALS inter-measurement period to 500ms
        self.write_8(i2c, 0x003E, 0x31).await?;
        // Configures interrupt on 'New Sample Ready threshold event'
        self.write_8(i2c, 0x0014, 0x24).await?;

        Ok(())
    }

    async fn write_8<T: I2c>(&self, i2c: &mut T, location: u16, byte: u8) -> Result<(), T::Error> {
        i2c.write(
            self.addr,
            &[
                ((location >> 8) & 0xFF) as u8,
                (location & 0xFF) as u8,
                byte,
            ],
        )
        .await
    }

    async fn read_8<T: I2c>(&self, i2c: &mut T, location: u16) -> Result<u8, T::Error> {
        let mut buf = [0];
        i2c.write_read(
            self.addr,
            &[((location >> 8) & 0xFF) as u8, (location & 0xFF) as u8],
            &mut buf,
        )
        .await?;
        Ok(buf[0])
    }
}
