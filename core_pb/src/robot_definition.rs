use crate::constants::{GU_PER_INCH, GU_PER_M};
use crate::drive_system::DriveSystem;
use crate::names::RobotName;
use core::f32::consts::PI;
use nalgebra::Rotation2;

#[derive(Copy, Clone, Debug)]
/// All the information that may vary from robot to robot
pub struct RobotDefinition<const WHEELS: usize> {
    /// Maximum radius of the circle the robot fits into
    pub radius: f32,

    /// Exposes methods to calculate motor velocities
    pub drive_system: DriveSystem<WHEELS>,
    /// Describes physical characteristics of the motors
    pub motors: [WheelDefinition; WHEELS],
    /// Default PID parameters - can change
    pub default_pid: [f32; 3],
    /// The maximum value for motor PWM pins
    pub pwm_top: u16,
    /// Which pwm pin corresponds to forwards and backwards for each motor - can change
    pub default_motor_config: [[usize; 2]; WHEELS],

    /// Whether the robot should expect to have access to a screen
    pub has_screen: bool,

    /// Maximum range of the sensors in meters
    pub sensor_distance: f32,
}

/// Describes physical characteristics of the motors
#[derive(Copy, Clone, Debug)]
pub struct WheelDefinition {}

impl RobotDefinition<3> {
    /// Create the default `RobotDefinition` for the given robot
    pub fn new(name: RobotName) -> Self {
        Self {
            radius: 0.715,

            drive_system: DriveSystem::new_omniwheel(
                0.019 * GU_PER_M,
                2.1 * GU_PER_INCH,
                [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0].map(Rotation2::new),
                [true, true, true],
            )
            .expect("Default robot drive definition couldn't be constructed"),
            motors: [WheelDefinition {}; 3],
            default_pid: if name.is_simulated() {
                [150.0, 0.0, 0.0]
            } else {
                [500.0, 20.0, 0.0]
            },
            pwm_top: 0x8000,
            default_motor_config: if name.is_simulated() {
                [[0, 1], [2, 3], [4, 5]]
            } else {
                [[5, 4], [2, 3], [0, 1]]
            },

            has_screen: false,
            sensor_distance: 1.5,
        }
    }
}
