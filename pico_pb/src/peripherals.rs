use crate::devices::bno08x::{ImuError, PacbotIMU};
use crate::devices::ltc2943::Ltc2943;
use crate::devices::seesaw_gamepad_qt::SeesawGamepadQt;
use crate::devices::ssd1306::{PacbotDisplay, PacbotDisplayWrapper};
use crate::devices::vl53l4cd::PacbotDistanceSensor;
use crate::{PacbotI2cBus, PicoRobotBehavior};
use core::sync::atomic::Ordering;
use core_pb::driving::peripherals::RobotPeripheralsBehavior;
use core_pb::messages::RobotButton;
use defmt::Format;
use display_interface::DisplayError;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_executor::task;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::i2c;
use futures::future::join4;
use portable_atomic::AtomicBool;
use vl53l4cd::Status;

/// number of distance sensors on the robot
pub const NUM_DIST_SENSORS: usize = 4;
/// what I2C addresses to reassign each distance sensor to
pub const DIST_SENSOR_ADDRESSES: [u8; NUM_DIST_SENSORS] = [0x31, 0x32, 0x33, 0x34];

pub async fn run_imu(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) -> ! {
    PacbotIMU::new(
        bus,
        enabled,
        &PicoRobotBehavior::get().enable_extra_imu_data,
        &PicoRobotBehavior::get().sig_angle,
    )
    .run_forever()
    .await
}

pub async fn run_dist(
    enabled: &'static AtomicBool,
    bus: &'static PacbotI2cBus,
    index: usize,
    xshut: AnyPin,
) -> ! {
    PacbotDistanceSensor::new(
        bus,
        Output::new(xshut, Level::Low),
        index,
        DIST_SENSOR_ADDRESSES[index],
        enabled,
        &PicoRobotBehavior::get().sig_distances[index],
    )
    .run_forever()
    .await
}

pub async fn run_battery_monitor(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) {
    Ltc2943::new(bus, enabled, &PicoRobotBehavior::get().sig_battery)
        .run_forever()
        .await
}

pub async fn run_gamepad(enabled: &'static AtomicBool, bus: &'static PacbotI2cBus) {
    SeesawGamepadQt::new(bus, enabled).run_forever().await
}

pub struct Peripherals {
    display: PacbotDisplayWrapper,
}

impl Peripherals {
    pub fn new(bus: &'static PacbotI2cBus) -> Self {
        Self {
            display: PacbotDisplayWrapper::new(bus),
        }
    }
}

#[derive(Clone, Debug, Format)]
#[allow(dead_code)]
pub enum PeripheralsError {
    Uninitialized,
    Disabled,
    Timeout,
    AwaitingMeasurement,
    DisplayError(DisplayError),
    DistanceSensorError(Option<Status>),
    ImuInitErr(ImuError),
    ImuError(ImuError),
    I2cError,
    BatteryMonitorError,
    Unimplemented,
}

impl From<I2cDeviceError<i2c::Error>> for PeripheralsError {
    fn from(_value: I2cDeviceError<i2c::Error>) -> Self {
        Self::I2cError
    }
}

impl RobotPeripheralsBehavior for Peripherals {
    type Display = PacbotDisplay;
    type Error = PeripheralsError;

    async fn draw_display<F>(&mut self, draw: F)
    where
        F: FnOnce(&mut Self::Display) -> Result<(), DisplayError>,
    {
        self.display.draw_display(draw).await;
    }

    async fn flip_screen(&mut self) {
        self.display.flush().await;
    }

    async fn read_button_event(&mut self) -> Option<(RobotButton, bool)> {
        for (i, b) in PicoRobotBehavior::get().buttons.iter().enumerate() {
            if b.load(Ordering::Relaxed) {
                b.store(false, Ordering::Relaxed);
                return Some((
                    match i {
                        0 => RobotButton::NorthX,
                        1 => RobotButton::WestY,
                        2 => RobotButton::EastA,
                        3 => RobotButton::SouthB,
                        4 => RobotButton::RightSelect,
                        5 => RobotButton::LeftStart,
                        _ => unreachable!(),
                    },
                    true,
                ));
            }
        }
        None
    }

    async fn read_joystick(&mut self) -> Option<(f32, f32)> {
        None
    }
}

#[task]
pub async fn manage_pico_i2c(bus: &'static PacbotI2cBus, xshut: [AnyPin; NUM_DIST_SENSORS]) {
    let data = PicoRobotBehavior::get();
    let [a, b, c, d] = xshut;
    join4(
        run_imu(&data.enable_imu, bus),
        join4(
            run_dist(&data.enable_dists, bus, 0, a),
            run_dist(&data.enable_dists, bus, 1, b),
            run_dist(&data.enable_dists, bus, 2, c),
            run_dist(&data.enable_dists, bus, 3, d),
        ),
        run_battery_monitor(&data.enable_battery_monitor, bus),
        run_gamepad(&data.enable_gamepad, bus),
    )
    .await;
}
