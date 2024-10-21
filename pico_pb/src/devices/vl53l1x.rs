//! Based on https://github.com/sparkfun/Qwiic_VL53L1X_Py/blob/master/qwiic_vl53l1x.py

use crate::devices::{read_u16, read_u32, read_u8, write_u16, write_u32, write_u8};
use constants::*;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

pub enum VL53L1XError<T: I2c> {
    I2cError(T::Error),
    CannotRead(T::Error),
    WrongFactoryId,
    InvalidTimingBudget(u16),
    InvalidDistanceMode(u8),
    SigmaFailed,
    SignalFailed,
    WrapAround,
    OtherSensorError(u8),
}

#[allow(dead_code)]
mod constants {
    pub const SOFT_RESET: u16 = 0x0000;
    pub const VL53L1_I2C_SLAVE__DEVICE_ADDRESS: u16 = 0x0001;
    pub const VL53L1_VHV_CONFIG__TIMEOUT_MACROP_LOOP_BOUND: u16 = 0x0008;
    pub const ALGO__CROSSTALK_COMPENSATION_PLANE_OFFSET_KCPS: u16 = 0x0016;
    pub const ALGO__CROSSTALK_COMPENSATION_X_PLANE_GRADIENT_KCPS: u16 = 0x0018;
    pub const ALGO__CROSSTALK_COMPENSATION_Y_PLANE_GRADIENT_KCPS: u16 = 0x001A;
    pub const ALGO__PART_TO_PART_RANGE_OFFSET_MM: u16 = 0x001E;
    pub const MM_CONFIG__INNER_OFFSET_MM: u16 = 0x0020;
    pub const MM_CONFIG__OUTER_OFFSET_MM: u16 = 0x0022;
    pub const GPIO_HV_MUX__CTRL: u16 = 0x0030;
    pub const GPIO__TIO_HV_STATUS: u16 = 0x0031;
    pub const SYSTEM__INTERRUPT_CONFIG_GPIO: u16 = 0x0046;
    pub const PHASECAL_CONFIG__TIMEOUT_MACROP: u16 = 0x004B;
    pub const RANGE_CONFIG__TIMEOUT_MACROP_A_HI: u16 = 0x005E;
    pub const RANGE_CONFIG__VCSEL_PERIOD_A: u16 = 0x0060;
    pub const RANGE_CONFIG__VCSEL_PERIOD_B: u16 = 0x0063;
    pub const RANGE_CONFIG__TIMEOUT_MACROP_B_HI: u16 = 0x0061;
    pub const RANGE_CONFIG__TIMEOUT_MACROP_B_LO: u16 = 0x0062;
    pub const RANGE_CONFIG__SIGMA_THRESH: u16 = 0x0064;
    pub const RANGE_CONFIG__MIN_COUNT_RATE_RTN_LIMIT_MCPS: u16 = 0x0066;
    pub const RANGE_CONFIG__VALID_PHASE_HIGH: u16 = 0x0069;
    pub const VL53L1_SYSTEM__INTERMEASUREMENT_PERIOD: u16 = 0x006C;
    pub const SYSTEM__THRESH_HIGH: u16 = 0x0072;
    pub const SYSTEM__THRESH_LOW: u16 = 0x0074;
    pub const SD_CONFIG__WOI_SD0: u16 = 0x0078;
    pub const SD_CONFIG__INITIAL_PHASE_SD0: u16 = 0x007A;
    pub const ROI_CONFIG__USER_ROI_CENTRE_SPAD: u16 = 0x007F;
    pub const ROI_CONFIG__USER_ROI_REQUESTED_GLOBAL_XY_SIZE: u16 = 0x0080;
    pub const SYSTEM__SEQUENCE_CONFIG: u16 = 0x0081;
    pub const VL53L1_SYSTEM__GROUPED_PARAMETER_HOLD: u16 = 0x0082;
    pub const SYSTEM__INTERRUPT_CLEAR: u16 = 0x0086;
    pub const SYSTEM__MODE_START: u16 = 0x0087;
    pub const VL53L1_RESULT__RANGE_STATUS: u16 = 0x0089;
    pub const VL53L1_RESULT__DSS_ACTUAL_EFFECTIVE_SPADS_SD0: u16 = 0x008C;
    pub const RESULT__AMBIENT_COUNT_RATE_MCPS_SD: u16 = 0x0090;
    pub const VL53L1_RESULT__FINAL_CROSSTALK_CORRECTED_RANGE_MM_SD0: u16 = 0x0096;
    pub const VL53L1_RESULT__PEAK_SIGNAL_COUNT_RATE_CROSSTALK_CORRECTED_MCPS_SD0: u16 = 0x0098;
    pub const VL53L1_RESULT__OSC_CALIBRATE_VAL: u16 = 0x00DE;
    pub const VL53L1_FIRMWARE__SYSTEM_STATUS: u16 = 0x00E5;
    pub const VL53L1_IDENTIFICATION__MODEL_ID: u16 = 0x010F;
    pub const VL53L1_ROI_CONFIG__MODE_ROI_CENTRE_SPAD: u16 = 0x013E;

    pub const _VL53L1X_DEFAULT_DEVICE_ADDRESS: u16 = 0x52;

    pub const VL51L1X_DEFAULT_CONFIGURATION: [u8; 91] = [
        0x00, // 0x2d : set bit 2 and 5 to 1 for fast plus mode (1MHz I2C), else don't touch
        0x01, // 0x2e : bit 0 if I2C pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD)
        0x01, // 0x2f : bit 0 if GPIO pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD)
        0x01, // 0x30 : set bit 4 to 0 for active high interrupt and 1 for active low (bits 3:0 must be 0x1), use set_interrupt_polarity()
        0x02, // 0x31 : bit 1 = interrupt depending on the polarity, use check_for_data_ready()
        0x00, // 0x32 : not user-modifiable
        0x02, // 0x33 : not user-modifiable
        0x08, // 0x34 : not user-modifiable
        0x00, // 0x35 : not user-modifiable
        0x08, // 0x36 : not user-modifiable
        0x10, // 0x37 : not user-modifiable
        0x01, // 0x38 : not user-modifiable
        0x01, // 0x39 : not user-modifiable
        0x00, // 0x3a : not user-modifiable
        0x00, // 0x3b : not user-modifiable
        0x00, // 0x3c : not user-modifiable
        0x00, // 0x3d : not user-modifiable
        0xff, // 0x3e : not user-modifiable
        0x00, // 0x3f : not user-modifiable
        0x0F, // 0x40 : not user-modifiable
        0x00, // 0x41 : not user-modifiable
        0x00, // 0x42 : not user-modifiable
        0x00, // 0x43 : not user-modifiable
        0x00, // 0x44 : not user-modifiable
        0x00, // 0x45 : not user-modifiable
        0x20, // 0x46 : interrupt configuration 0->level low detection, 1-> level high, 2-> Out of window, 3->In window, 0x20-> New sample ready , TBC
        0x0b, // 0x47 : not user-modifiable
        0x00, // 0x48 : not user-modifiable
        0x00, // 0x49 : not user-modifiable
        0x02, // 0x4a : not user-modifiable
        0x0a, // 0x4b : not user-modifiable
        0x21, // 0x4c : not user-modifiable
        0x00, // 0x4d : not user-modifiable
        0x00, // 0x4e : not user-modifiable
        0x05, // 0x4f : not user-modifiable
        0x00, // 0x50 : not user-modifiable
        0x00, // 0x51 : not user-modifiable
        0x00, // 0x52 : not user-modifiable
        0x00, // 0x53 : not user-modifiable
        0xc8, // 0x54 : not user-modifiable
        0x00, // 0x55 : not user-modifiable
        0x00, // 0x56 : not user-modifiable
        0x38, // 0x57 : not user-modifiable
        0xff, // 0x58 : not user-modifiable
        0x01, // 0x59 : not user-modifiable
        0x00, // 0x5a : not user-modifiable
        0x08, // 0x5b : not user-modifiable
        0x00, // 0x5c : not user-modifiable
        0x00, // 0x5d : not user-modifiable
        0x01, // 0x5e : not user-modifiable
        0xdb, // 0x5f : not user-modifiable
        0x0f, // 0x60 : not user-modifiable
        0x01, // 0x61 : not user-modifiable
        0xf1, // 0x62 : not user-modifiable
        0x0d, // 0x63 : not user-modifiable
        0x01, // 0x64 : Sigma threshold MSB (mm in 14.2 format for MSB+LSB), use set_sigma_threshold(), default value 90 mm
        0x68, // 0x65 : Sigma threshold LSB
        0x00, // 0x66 : Min count Rate MSB (MCPS in 9.7 format for MSB+LSB), use set_signal_threshold()
        0x80, // 0x67 : Min count Rate LSB
        0x08, // 0x68 : not user-modifiable
        0xb8, // 0x69 : not user-modifiable
        0x00, // 0x6a : not user-modifiable
        0x00, // 0x6b : not user-modifiable
        0x00, // 0x6c : Intermeasurement period MSB, 32 bits register, use set_inter_measurement_in_ms()
        0x00, // 0x6d : Intermeasurement period
        0x0f, // 0x6e : Intermeasurement period
        0x89, // 0x6f : Intermeasurement period LSB
        0x00, // 0x70 : not user-modifiable
        0x00, // 0x71 : not user-modifiable
        0x00, // 0x72 : distance threshold high MSB (in mm, MSB+LSB), use SetD:tanceThreshold()
        0x00, // 0x73 : distance threshold high LSB
        0x00, // 0x74 : distance threshold low MSB ( in mm, MSB+LSB), use SetD:tanceThreshold()
        0x00, // 0x75 : distance threshold low LSB
        0x00, // 0x76 : not user-modifiable
        0x01, // 0x77 : not user-modifiable
        0x0f, // 0x78 : not user-modifiable
        0x0d, // 0x79 : not user-modifiable
        0x0e, // 0x7a : not user-modifiable
        0x0e, // 0x7b : not user-modifiable
        0x00, // 0x7c : not user-modifiable
        0x00, // 0x7d : not user-modifiable
        0x02, // 0x7e : not user-modifiable
        0xc7, // 0x7f : ROI center, use set_roi()
        0xff, // 0x80 : XY ROI (X=Width, Y=Height), use set_roi()
        0x9B, // 0x81 : not user-modifiable
        0x00, // 0x82 : not user-modifiable
        0x00, // 0x83 : not user-modifiable
        0x00, // 0x84 : not user-modifiable
        0x01, // 0x85 : not user-modifiable
        0x00, // 0x86 : clear interrupt, use clear_interrupt()
        0x00, // 0x87 : start ranging, use start_ranging() or stop_ranging(), If you want an automatic start after self.init() call, put 0x40 in location 0x87
    ];

    pub const VL53L1_ERROR_NONE: i16 = 0;
    /// Warning invalid calibration data may be in used
    ///     VL53L1_InitData()
    ///     VL53L1_GetOffsetCalibrationData
    ///     VL53L1_SetOffsetCalibrationData
    pub const VL53L1_ERROR_CALIBRATION_WARNING: i16 = -1;
    /// Warning parameter passed was clipped to min before to be applied
    pub const VL53L1_ERROR_MIN_CLIPPED: i16 = -2;

    /// Unqualified error
    pub const VL53L1_ERROR_UNDEFINED: i16 = -3;
    /// Parameter passed is invalid or out of range
    pub const VL53L1_ERROR_INVALID_PARAMS: i16 = -4;
    /// Function is not supported in current mode or configuration
    pub const VL53L1_ERROR_NOT_SUPPORTED: i16 = -5;
    /// Device report a ranging error interrupt status
    pub const VL53L1_ERROR_RANGE_ERROR: i16 = -6;
    /// Aborted due to time out
    pub const VL53L1_ERROR_TIME_OUT: i16 = -7;
    /// Asked mode is not supported by the device
    pub const VL53L1_ERROR_MODE_NOT_SUPPORTED: i16 = -8;
    /// ...
    pub const VL53L1_ERROR_BUFFER_TOO_SMALL: i16 = -9;
    /// Supplied buffer is larger than I2C supports
    pub const VL53L1_ERROR_COMMS_BUFFER_TOO_SMALL: i16 = -10;
    /// User tried to setup a non-existing GPIO pin
    pub const VL53L1_ERROR_GPIO_NOT_EXISTING: i16 = -11;
    /// unsupported GPIO functionality
    pub const VL53L1_ERROR_GPIO_FUNCTIONALITY_NOT_SUPPORTED: i16 = -12;
    /// error reported from IO functions
    pub const VL53L1_ERROR_CONTROL_INTERFACE: i16 = -13;
    /// The command is not allowed in the current device state (power down)
    pub const VL53L1_ERROR_INVALID_COMMAND: i16 = -14;
    /// In the function a division by zero occurs
    pub const VL53L1_ERROR_DIVISION_BY_ZERO: i16 = -15;
    /// Error during reference SPAD initialization
    pub const VL53L1_ERROR_REF_SPAD_INIT: i16 = -16;
    /// GPH sync interrupt check fail - API out of sync with device
    pub const VL53L1_ERROR_GPH_SYNC_CHECK_FAIL: i16 = -17;
    /// Stream count check fail - API out of sync with device
    pub const VL53L1_ERROR_STREAM_COUNT_CHECK_FAIL: i16 = -18;
    /// GPH ID check fail - API out of sync with device
    pub const VL53L1_ERROR_GPH_ID_CHECK_FAIL: i16 = -19;
    /// Zone dynamic config stream count check failed - API out of sync
    pub const VL53L1_ERROR_ZONE_STREAM_COUNT_CHECK_FAIL: i16 = -20;
    /// Zone dynamic config GPH ID check failed - API out of sync
    pub const VL53L1_ERROR_ZONE_GPH_ID_CHECK_FAIL: i16 = -21;

    /// Thrown when run_xtalk_extraction fn has 0 succesful samples when using
    /// the full array to sample the xtalk. In this case there is not enough
    /// information to generate new Xtalk parm info. The function will exit and
    /// leave the current xtalk parameters unaltered
    pub const VL53L1_ERROR_XTALK_EXTRACTION_NO_SAMPLE_FAI: i16 = -22;
    /// Thrown when run_xtalk_extraction fn has found that the avg sigma
    /// estimate of the full array xtalk sample is > than the maximal limit
    /// allowed. In this case the xtalk sample is too noisy for measurement.
    /// The function will exit and leave the current xtalk parameters unaltered.
    pub const VL53L1_ERROR_XTALK_EXTRACTION_SIGMA_LIMIT_FAIL: i16 = -23;

    /// Thrown if there one of stages has no valid offset calibration
    /// samples. A fatal error calibration not valid
    pub const VL53L1_ERROR_OFFSET_CAL_NO_SAMPLE_FAIL: i16 = -24;
    /// Thrown if there one of stages has zero effective SPADS Traps the case
    /// when MM1 SPADs is zero. A fatal error calibration not valid
    pub const VL53L1_ERROR_OFFSET_CAL_NO_SPADS_ENABLED_FAIL: i16 = -25;
    /// Thrown if then some of the zones have no valid samples. A fatal error
    /// calibration not valid
    pub const VL53L1_ERROR_ZONE_CAL_NO_SAMPLE_FAIL: i16 = -26;
    /// Thrown if the tuning file key table version does not match with
    /// expected value. The driver expects the key table version to match the
    /// compiled default version number in the define
    /// VL53L1_TUNINGPARM_KEY_TABLE_VERSION_DEFAULT*
    pub const VL53L1_ERROR_TUNING_PARM_KEY_MISMATCH: i16 = -27;
    /// Thrown if there are less than 5 good SPADs are available.
    pub const VL53L1_WARNING_REF_SPAD_CHAR_NOT_ENOUGH_SPADS: i16 = -28;
    /// Thrown if the final reference rate is greater than the upper reference
    /// rate limit - default is 40 Mcps. Implies a minimum Q3 (x10) SPAD (5)
    /// selected
    pub const VL53L1_WARNING_REF_SPAD_CHAR_RATE_TOO_HIGH: i16 = -29;
    /// Thrown if the final reference rate is less than the lower reference
    /// rate limit - default is 10 Mcps. Implies maximum Q1 (x1) SPADs selected
    pub const VL53L1_WARNING_REF_SPAD_CHAR_RATE_TOO_LOW: i16 = -30;

    /// Thrown if there is less than the requested number of valid samples.
    pub const VL53L1_WARNING_OFFSET_CAL_MISSING_SAMPLES: i16 = -31;
    /// Thrown if the offset calibration range sigma estimate is greater than
    /// 8.0 mm. This is the recommended min value to yield a stable offset
    /// measurement
    pub const VL53L1_WARNING_OFFSET_CAL_SIGMA_TOO_HIGH: i16 = -32;
    /// Thrown when VL53L1_run_offset_calibration() peak rate is greater than
    /// that 50.0Mcps. This is the recommended max rate to avoid pile-up
    /// influencing the offset measurement
    pub const VL53L1_WARNING_OFFSET_CAL_RATE_TOO_HIGH: i16 = -33;
    /// Thrown when VL53L1_run_offset_calibration() when one of stages range
    /// has less that 5.0 effective SPADS. This is the recommended min value to
    /// yield a stable offset
    pub const VL53L1_WARNING_OFFSET_CAL_SPAD_COUNT_TOO_LOW: i16 = -34;

    /// Thrown if one of more of the zones have less than the requested number
    /// of valid samples
    pub const VL53L1_WARNING_ZONE_CAL_MISSING_SAMPLES: i16 = -35;
    /// Thrown if one or more zones have sigma estimate value greater than
    /// 8.0 mm. This is the recommended min value to yield a stable offset
    /// measurement
    pub const VL53L1_WARNING_ZONE_CAL_SIGMA_TOO_HIGH: i16 = -36;
    /// Thrown if one of more zones have peak rate higher than that 50.0Mcps.
    /// This is the recommended max rate to avoid pile-up influencing the offset
    /// measurement
    pub const VL53L1_WARNING_ZONE_CAL_RATE_TOO_HIGH: i16 = -37;

    /// Thrown to notify that some of the xtalk samples did not yield valid
    /// ranging pulse data while attempting to measure the xtalk signal in
    /// vl53l1_run_xtalk_extract(). This can signify any of the zones are missing
    /// samples, for further debug information the xtalk_results struct should be
    /// referred to. This warning is for notification only, the xtalk pulse and
    /// shape have still been generated
    pub const VL53L1_WARNING_XTALK_MISSING_SAMPLES: i16 = -38;
    /// Thrown to notify that some of teh xtalk samples used for gradient
    /// generation did not yield valid ranging pulse data while attempting to
    /// measure the xtalk signal in vl53l1_run_xtalk_extract(). This can signify
    /// that any one of the zones 0-3 yielded no successful samples. The
    /// xtalk_results struct should be referred to for further debug info. This
    /// warning is for notification only, the xtalk pulse and shape have still
    /// been generated.
    pub const VL53L1_WARNING_XTALK_NO_SAMPLES_FOR_GRADIENT: i16 = -39;
    /// Thrown to notify that some of the xtalk samples used for gradient
    /// generation did not pass the sigma limit check while attempting to
    /// measure the xtalk signal in vl53l1_run_xtalk_extract(). This can signify
    /// that any one of the zones 0-3 yielded an avg sigma_mm value > the limit.
    /// The xtalk_results struct should be referred to for further debug info.
    /// This warning is for notification only, the xtalk pulse and shape have
    /// still been generated.
    pub const VL53L1_WARNING_XTALK_SIGMA_LIMIT_FOR_GRADIENT: i16 = -40;
    /// Tells requested functionality has not been implemented yet or not
    /// compatible with the device
    pub const VL53L1_ERROR_NOT_IMPLEMENTED: i16 = -41;
    /// Tells the starting code for platform
    ///      VL53L1_define_Error_group
    pub const VL53L1_ERROR_PLATFORM_SPECIFIC_START: i16 = -60;
}

/// Manages a VL53L1X distance sensor
pub struct VL53L1X {
    addr: u8,
}

#[allow(dead_code)]
impl VL53L1X {
    /// Creates a new VL53L1X and initializes it
    pub async fn new<T: I2c>(i2c: &mut T, addr: u8) -> Result<Self, VL53L1XError<T>> {
        let mut s = Self { addr };
        s.try_initialize(i2c).await?;
        Ok(s)
    }

    /// This function returns the distance measured by the sensor in mm
    pub async fn get_distance<T: I2c>(&mut self, i2c: &mut T) -> Result<u16, VL53L1XError<T>> {
        self.read_u16(i2c, VL53L1_RESULT__FINAL_CROSSTALK_CORRECTED_RANGE_MM_SD0)
            .await
    }

    /// This function sets the sensor I2C address used in case multiple devices application,
    /// default address **0x29** (0x52 >> 1)
    pub async fn set_address<T: I2c>(
        &mut self,
        i2c: &mut T,
        new_addr: u8,
    ) -> Result<(), VL53L1XError<T>> {
        self.write_u8(i2c, VL53L1_I2C_SLAVE__DEVICE_ADDRESS, new_addr)
            .await?;
        self.addr = new_addr;
        Ok(())
    }

    /// This function clears the interrupt, to be called after a ranging data reading to arm the
    /// interrupt for the next data ready event.
    pub async fn clear_interrupt<T: I2c>(&mut self, i2c: &mut T) -> Result<(), VL53L1XError<T>> {
        self.write_u8(i2c, SYSTEM__INTERRUPT_CLEAR, 0x01).await
    }

    // TODO set_interrupt_polarity

    /// This function returns the interrupt polarity
    ///
    /// True = active high, False = active low
    pub async fn get_interrupt_polarity<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<bool, VL53L1XError<T>> {
        Ok((self.read_u8(i2c, GPIO_HV_MUX__CTRL).await? & 0x10) >> 4 == 0)
    }

    /// This function starts the ranging distance operation
    ///
    /// The ranging operation is continuous. The clear interrupt has to be done after each get data
    /// to allow the interrupt to raise when the next data is ready 1=active high (**default**),
    /// 0=active low, use set_interrupt_polarity() to change the interrupt polarity if required.
    pub async fn start_ranging<T: I2c>(&mut self, i2c: &mut T) -> Result<(), VL53L1XError<T>> {
        self.write_u8(i2c, SYSTEM__MODE_START, 0x40).await
    }

    /// This function stops the ranging.
    pub async fn stop_ranging<T: I2c>(&mut self, i2c: &mut T) -> Result<(), VL53L1XError<T>> {
        self.write_u8(i2c, SYSTEM__MODE_START, 0x00).await
    }

    /// This function checks if the new ranging data is available by polling the dedicated register.
    pub async fn check_for_data_ready<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<bool, VL53L1XError<T>> {
        let int_pol = if self.get_interrupt_polarity(i2c).await? {
            1
        } else {
            0
        };
        let temp = self.read_u8(i2c, GPIO__TIO_HV_STATUS).await?;

        Ok((temp & 1) == int_pol)
    }

    /// This function programs the timing budget in ms.
    ///
    /// Predefined values = 15, 20, 33, 50, 100 (**default**), 200, 500.
    pub async fn set_timing_budget_ms<T: I2c>(
        &mut self,
        i2c: &mut T,
        timing_budget: u16,
    ) -> Result<(), VL53L1XError<T>> {
        let (a, b) = match (self.get_distance_mode(i2c).await?, timing_budget) {
            (true, 15) => (0x01D, 0x0027),
            (true, 20) => (0x0051, 0x006E),
            (true, 33) => (0x00D6, 0x006E),
            (true, 50) => (0x1AE, 0x01E8),
            (true, 100) => (0x02E1, 0x0388),
            (true, 200) => (0x03E1, 0x0496),
            (true, 500) => (0x0591, 0x05C1),
            (false, 20) => (0x001E, 0x0022),
            (false, 33) => (0x0060, 0x006E),
            (false, 50) => (0x00AD, 0x00C6),
            (false, 100) => (0x01CC, 0x01EA),
            (false, 200) => (0x02D9, 0x02F8),
            (false, 500) => (0x048F, 0x04A4),
            (_, _) => return Err(VL53L1XError::InvalidTimingBudget(timing_budget)),
        };
        self.write_u16(i2c, RANGE_CONFIG__TIMEOUT_MACROP_A_HI, a)
            .await?;
        self.write_u16(i2c, RANGE_CONFIG__TIMEOUT_MACROP_B_HI, b)
            .await
    }

    /// This function returns the current timing budget in ms.
    pub async fn get_timing_budget_in_ms<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<u16, VL53L1XError<T>> {
        let temp = self
            .read_u16(i2c, RANGE_CONFIG__TIMEOUT_MACROP_A_HI)
            .await?;

        match temp {
            0x001D => Ok(15),
            0x0051 => Ok(20),
            0x001E => Ok(20),
            0x00D6 => Ok(33),
            0x0060 => Ok(33),
            0x1AE => Ok(50),
            0x00AD => Ok(50),
            0x02E1 => Ok(100),
            0x01CC => Ok(100),
            0x03E1 => Ok(200),
            0x02D9 => Ok(200),
            0x0591 => Ok(500),
            0x048F => Ok(500),
            t => Err(VL53L1XError::InvalidTimingBudget(t)),
        }
    }

    /// This function programs the distance mode (1=short, 2=long(default)).
    ///
    /// 1- Short mode max distance is limited to 1.3 m but better ambient immunity.
    /// 2- Long mode can range up to 4 m in the dark with 200 ms timing budget (**default**).
    pub async fn set_distance_mode<T: I2c>(
        &mut self,
        i2c: &mut T,
        short_range: bool,
    ) -> Result<(), VL53L1XError<T>> {
        let timing_budget = self.get_timing_budget_in_ms(i2c).await?;

        if short_range {
            self.write_u8(i2c, PHASECAL_CONFIG__TIMEOUT_MACROP, 0x14)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VCSEL_PERIOD_A, 0x07)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VCSEL_PERIOD_B, 0x05)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VALID_PHASE_HIGH, 0x38)
                .await?;
            self.write_u16(i2c, SD_CONFIG__WOI_SD0, 0x0705).await?;
            self.write_u16(i2c, SD_CONFIG__INITIAL_PHASE_SD0, 0x0606)
                .await?;
        } else {
            self.write_u8(i2c, PHASECAL_CONFIG__TIMEOUT_MACROP, 0x0A)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VCSEL_PERIOD_A, 0x0F)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VCSEL_PERIOD_B, 0x0D)
                .await?;
            self.write_u8(i2c, RANGE_CONFIG__VALID_PHASE_HIGH, 0xB8)
                .await?;
            self.write_u16(i2c, SD_CONFIG__WOI_SD0, 0x0F0D).await?;
            self.write_u16(i2c, SD_CONFIG__INITIAL_PHASE_SD0, 0x0E0E)
                .await?;
        }

        self.set_timing_budget_ms(i2c, timing_budget).await
    }

    /// This function returns the current distance mode (1=short, 2=long).
    ///
    /// true - Short mode max distance is limited to 1.3 m but better ambient immunity.
    /// false - Long mode can range up to 4 m in the dark with 200 ms timing budget (**default**).
    pub async fn get_distance_mode<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<bool, VL53L1XError<T>> {
        match self.read_u8(i2c, PHASECAL_CONFIG__TIMEOUT_MACROP).await? {
            0x14 => Ok(true),
            0x0A => Ok(false),
            m => Err(VL53L1XError::InvalidDistanceMode(m)),
        }
    }

    /// This function programs the Intermeasurement period in ms.
    ///
    /// Intermeasurement period must be >/= timing budget. This condition is not checked by the API,
    /// the customer has the duty to check the condition. **Default = 100 ms**
    // todo pub async fn set_inter_measurement_in_ms<T: I2c>(
    //     &mut self,
    //     i2c: &mut T,
    //     period: f32,
    // ) -> Result<(), VL53L1XError<T>> {
    //     let clock_pll =
    //         (self.read_u16(i2c, VL53L1_RESULT__OSC_CALIBRATE_VAL).await? & 0x3FF) as f32;
    //
    //     // todo
    //     // self.write_u32(
    //     //     i2c,
    //     //     VL53L1_SYSTEM__INTERMEASUREMENT_PERIOD,
    //     //     u32::from((clock_pll * period * 1.075).floor()),
    //     // )
    // }

    /// This function returns the Intermeasurement period in ms.
    pub async fn get_inter_measurement_in_ms<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<f32, VL53L1XError<T>> {
        let tmp = self
            .read_u32(i2c, VL53L1_SYSTEM__INTERMEASUREMENT_PERIOD)
            .await? as f32;
        let clock_pll = self.read_u16(i2c, VL53L1_RESULT__OSC_CALIBRATE_VAL).await? as f32;

        Ok(tmp / (clock_pll * 1.065))
    }

    /// This function returns the boot state of the device
    pub async fn boot_state<T: I2c>(&mut self, i2c: &mut T) -> Result<bool, VL53L1XError<T>> {
        Ok(self.read_u8(i2c, VL53L1_FIRMWARE__SYSTEM_STATUS).await? != 0)
    }

    /// This function returns the returned signal per SPAD in kcps/SPAD
    /// (kcps stands for Kilo Count Per Second).
    pub async fn get_signal_per_spad<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<f32, VL53L1XError<T>> {
        let signal = self
            .read_u16(
                i2c,
                VL53L1_RESULT__PEAK_SIGNAL_COUNT_RATE_CROSSTALK_CORRECTED_MCPS_SD0,
            )
            .await? as f32;
        let sp_nb = self
            .read_u16(i2c, VL53L1_RESULT__DSS_ACTUAL_EFFECTIVE_SPADS_SD0)
            .await? as f32;

        Ok(2000.0 * signal / sp_nb)
    }

    /// This function returns the ambient per SPAD in kcps/SPAD
    pub async fn get_ambient_per_spad<T: I2c>(
        &mut self,
        i2c: &mut T,
    ) -> Result<f32, VL53L1XError<T>> {
        let ambient_rate = self
            .read_u16(i2c, RESULT__AMBIENT_COUNT_RATE_MCPS_SD)
            .await? as f32;
        let sp_nb = self
            .read_u16(i2c, VL53L1_RESULT__DSS_ACTUAL_EFFECTIVE_SPADS_SD0)
            .await? as f32;

        Ok(2000.0 * ambient_rate / sp_nb)
    }

    /// This function returns the returned signal in kcps.
    pub async fn get_signal_rate<T: I2c>(&mut self, i2c: &mut T) -> Result<u16, VL53L1XError<T>> {
        let tmp = self
            .read_u16(
                i2c,
                VL53L1_RESULT__PEAK_SIGNAL_COUNT_RATE_CROSSTALK_CORRECTED_MCPS_SD0,
            )
            .await?;

        Ok(tmp * 8)
    }

    /// This function returns the current number of enabled SPADs
    pub async fn get_spad_nb<T: I2c>(&mut self, i2c: &mut T) -> Result<u16, VL53L1XError<T>> {
        let tmp = self
            .read_u16(i2c, VL53L1_RESULT__DSS_ACTUAL_EFFECTIVE_SPADS_SD0)
            .await?;

        Ok(tmp >> 8)
    }

    /// This function returns the ambient rate in kcps
    pub async fn get_ambient_rate<T: I2c>(&mut self, i2c: &mut T) -> Result<u16, VL53L1XError<T>> {
        let tmp = self
            .read_u16(i2c, RESULT__AMBIENT_COUNT_RATE_MCPS_SD)
            .await?;

        Ok(tmp * 8)
    }

    /// This function returns the ranging status error
    async fn get_range_status<T: I2c>(&mut self, i2c: &mut T) -> Result<(), VL53L1XError<T>> {
        let rg_st = self.read_u8(i2c, VL53L1_RESULT__RANGE_STATUS).await? & 0x1F;

        match rg_st {
            9 => Ok(()),
            6 => Err(VL53L1XError::SigmaFailed),
            4 => Err(VL53L1XError::SignalFailed),
            8 => Err(VL53L1XError::OtherSensorError(3)),
            5 => Err(VL53L1XError::OtherSensorError(4)),
            3 => Err(VL53L1XError::OtherSensorError(5)),
            19 => Err(VL53L1XError::OtherSensorError(6)),
            7 => Err(VL53L1XError::WrapAround),
            12 => Err(VL53L1XError::OtherSensorError(9)),
            18 => Err(VL53L1XError::OtherSensorError(10)),
            22 => Err(VL53L1XError::OtherSensorError(11)),
            23 => Err(VL53L1XError::OtherSensorError(12)),
            13 => Err(VL53L1XError::OtherSensorError(13)),
            x => Err(VL53L1XError::OtherSensorError(x)),
        }
    }

    /// This function programs the offset correction in mm
    pub async fn set_offset<T: I2c>(
        &mut self,
        i2c: &mut T,
        offset: u16,
    ) -> Result<(), VL53L1XError<T>> {
        let tmp = offset * 4;

        self.write_u16(i2c, ALGO__PART_TO_PART_RANGE_OFFSET_MM, tmp)
            .await?;
        self.write_u16(i2c, MM_CONFIG__INNER_OFFSET_MM, 0x0).await?;
        self.write_u16(i2c, MM_CONFIG__OUTER_OFFSET_MM, 0x0).await
    }

    // todo next get_offset

    /// This function loads the 135 bytes default values to initialize the sensor.
    async fn try_initialize<T: I2c>(&mut self, i2c: &mut T) -> Result<bool, VL53L1XError<T>> {
        loop {
            if self.boot_state(i2c).await? {
                break;
            } else {
                Timer::after_millis(2).await;
            }
        }
        self.assert_factory_id(i2c).await?;
        for addr in 0x2D..0x87 + 1 {
            self.write_u8(
                i2c,
                addr,
                VL51L1X_DEFAULT_CONFIGURATION[(addr - 0x2D) as usize],
            )
            .await?;
        }
        Ok(true)
    }

    /// Read function of the ID device. (Verifies id matches factory number)
    async fn assert_factory_id<T: I2c>(&mut self, i2c: &mut T) -> Result<(), VL53L1XError<T>> {
        match read_u16(self.addr, i2c, VL53L1_IDENTIFICATION__MODEL_ID).await {
            Ok(0xEEAC) => Ok(()),
            Ok(_) => Err(VL53L1XError::WrongFactoryId),
            Err(e) => Err(VL53L1XError::CannotRead(e)),
        }
    }

    /// Read from the I2c bus and wrap errors with VL53L1XError
    async fn read_u8<T: I2c>(&mut self, i2c: &mut T, register: u16) -> Result<u8, VL53L1XError<T>> {
        read_u8(self.addr, i2c, register)
            .await
            .map_err(|e| VL53L1XError::CannotRead(e))
    }

    /// Read from the I2c bus and wrap errors with VL53L1XError
    async fn read_u16<T: I2c>(
        &mut self,
        i2c: &mut T,
        register: u16,
    ) -> Result<u16, VL53L1XError<T>> {
        read_u16(self.addr, i2c, register)
            .await
            .map_err(|e| VL53L1XError::CannotRead(e))
    }

    /// Read from the I2c bus and wrap errors with VL53L1XError
    async fn read_u32<T: I2c>(
        &mut self,
        i2c: &mut T,
        register: u16,
    ) -> Result<u32, VL53L1XError<T>> {
        read_u32(self.addr, i2c, register)
            .await
            .map_err(|e| VL53L1XError::CannotRead(e))
    }

    /// Write to the I2c bus and wrap errors with VL53L1XError
    async fn write_u8<T: I2c>(
        &mut self,
        i2c: &mut T,
        register: u16,
        data: u8,
    ) -> Result<(), VL53L1XError<T>> {
        write_u8(self.addr, i2c, register, data)
            .await
            .map_err(|x| VL53L1XError::I2cError(x))
    }

    /// Write to the I2c bus and wrap errors with VL53L1XError
    async fn write_u16<T: I2c>(
        &mut self,
        i2c: &mut T,
        register: u16,
        data: u16,
    ) -> Result<(), VL53L1XError<T>> {
        write_u16(self.addr, i2c, register, data)
            .await
            .map_err(|x| VL53L1XError::I2cError(x))
    }

    /// Write to the I2c bus and wrap errors with VL53L1XError
    async fn write_u32<T: I2c>(
        &mut self,
        i2c: &mut T,
        register: u16,
        data: u32,
    ) -> Result<(), VL53L1XError<T>> {
        write_u32(self.addr, i2c, register, data)
            .await
            .map_err(|x| VL53L1XError::I2cError(x))
    }
}
