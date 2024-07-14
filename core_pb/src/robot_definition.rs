use crate::constants::{INCHES_PER_GU, MM_PER_GU};
use crate::drive_system::DriveSystem;
use nalgebra::Rotation2;
use std::f32::consts::PI;

#[derive(Copy, Clone, Debug)]
/// All the information that may vary from robot to robot
pub struct RobotDefinition<const WHEELS: usize> {
    /// Maximum radius of the circle the robot fits into
    pub radius: f32,

    /// Exposes methods to calculate motor velocities
    pub drive_system: DriveSystem<WHEELS>,
    /// Describes physical characteristics of the motors
    pub motors: [WheelDefinition; WHEELS],

    /// Whether the robot should expect to have access to a screen
    pub has_screen: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct WheelDefinition {}

impl Default for RobotDefinition<3> {
    fn default() -> Self {
        Self {
            radius: 0.715,

            drive_system: DriveSystem::new_omniwheel(
                19.0 * MM_PER_GU,
                2.1 * INCHES_PER_GU,
                [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0].map(|a| Rotation2::new(a)),
                [true, true, true],
            )
            .expect("Default robot drive definition couldn't be constructed"),
            motors: [WheelDefinition {}; 3],

            has_screen: false,
        }
    }
}
