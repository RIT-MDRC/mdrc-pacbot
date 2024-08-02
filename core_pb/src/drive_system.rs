//! Systems for motor speed calculations
//!
//! Note: one must be especially careful about length units when working with [`DriveSystem`].
//! Ensure all lengths are in grid units (gu)

use core::f32::consts::{FRAC_PI_2, FRAC_PI_6, PI};
#[cfg(not(feature = "std"))]
use micromath::F32Ext;
use nalgebra::{Rotation2, Vector2};

#[derive(Copy, Clone, Debug)]
pub enum DriveSystem<const WHEELS: usize> {
    /// A drive system with any number of omniwheels that can freely move perpendicularly to their
    /// primary direction using rollers
    Omniwheel {
        wheel_radius: f32,
        robot_radius: f32,
        radius_angles_rad: [Rotation2<f32>; WHEELS],
        forwards_is_clockwise: [bool; WHEELS],
    },
}

impl<const WHEELS: usize> DriveSystem<WHEELS> {
    /// A drive system with any number of omniwheels that can freely move perpendicularly to their
    /// primary direction using rollers
    ///
    /// Length and speeds should use grid units (gu)
    ///
    /// # Arguments
    ///
    /// - wheel_radius: the radius from the center of a wheel to its edge, in gu; must be positive
    /// - robot_radius: 2d distance from the center of the robot to the center of each wheel, in gu; must be positive
    /// - radius_angles_rad: Angle of the line from the center of the robot to the center of each wheel, in radians, relative to the robot
    /// - forwards_is_clockwise: For each motor, if it is driven forwards, does that result in the wheel turning clockwise?
    ///
    /// Note: if all wheels are turning clockwise, the robot as a whole turn rotate counterclockwise
    ///
    /// # Returns
    ///
    /// DriveSystem if the configuration is valid, otherwise None
    pub fn new_omniwheel(
        wheel_radius: f32,
        robot_radius: f32,
        radius_angles_rad: [Rotation2<f32>; WHEELS],
        forwards_is_clockwise: [bool; WHEELS],
    ) -> Option<DriveSystem<WHEELS>> {
        if robot_radius <= 0.0 || wheel_radius <= 0.0 {
            return None;
        }

        Some(Self::Omniwheel {
            wheel_radius,
            robot_radius,
            radius_angles_rad,
            forwards_is_clockwise,
        })
    }

    /// Get the speeds that each motor should turn for the given targets, in rad/s
    ///
    /// # Arguments
    ///
    /// - target_velocity: the desired velocity of the robot in gu/s, relative to the robot
    /// - target_angular_velocity: the desired angular velocity of the robot in rad/s, clockwise positive
    pub fn get_motor_speed_omni(
        &self,
        target_velocity: Vector2<f32>,
        target_angular_velocity: f32,
    ) -> [f32; WHEELS] {
        match self {
            DriveSystem::Omniwheel {
                wheel_radius,
                radius_angles_rad,
                robot_radius,
                forwards_is_clockwise,
            } => {
                // this is the speed at which a point on the edge of the robot moves due
                // to the desired angular velocity
                let robot_edge_speed = target_angular_velocity * *robot_radius;
                // this is the speed each wheel should turn forwards to achieve this
                let target_angular_velocity_wheel_speed = robot_edge_speed / *wheel_radius;

                let target_angle = target_velocity.y.atan2(target_velocity.x);
                let target_speed = target_velocity.magnitude();
                let mut i = 0;
                radius_angles_rad.map(|radius_angle| {
                    // this is the direction the robot would move if only this motor mattered, going forwards
                    let forwards_direction = radius_angle
                        * if forwards_is_clockwise[i] {
                            Rotation2::new(PI / 2.0)
                        } else {
                            Rotation2::new(-PI / 2.0)
                        };
                    // this is the difference between the direction we want to go and the direction
                    // that this wheel is pointing
                    let difference_angle = target_angle - forwards_direction.angle();
                    // this number is:
                    // 1 if the wheel is pointing where we want to go
                    // -1 if the wheel is pointing opposite where we want to go
                    // positive if the wheel is pointing at least partially where we want to go
                    // 0 if the wheel is perpendicular to where we want to go
                    let contribution = difference_angle.cos();
                    // this is how fast this wheel should move linearly along the ground
                    let linear_velocity = contribution * target_speed;
                    // this is how fast the motor should turn
                    let angular_velocity = linear_velocity / *wheel_radius;
                    // add velocity from robot spinning
                    let final_answer = angular_velocity
                        + if forwards_is_clockwise[i] {
                            target_angular_velocity_wheel_speed
                        } else {
                            -target_angular_velocity_wheel_speed
                        };
                    i += 1;
                    final_answer
                })
            }
        }
    }
}

impl DriveSystem<3> {
    /// Given signed motor speeds, find the velocity and angular velocity of the robot
    ///
    /// # Arguments
    ///
    /// - motor_speeds: the signed speeds of the motors, in rad/s
    pub fn get_actual_vel_omni(&self, motor_speeds: [f32; 3]) -> (Vector2<f32>, f32) {
        match self {
            DriveSystem::Omniwheel {
                wheel_radius,
                robot_radius,
                forwards_is_clockwise,
                ..
            } => {
                // rotational to linear
                let rot_to_lin = |v: f32, fic: bool| {
                    if fic {
                        v * *wheel_radius
                    } else {
                        -v * *wheel_radius
                    }
                };
                let v_a = rot_to_lin(motor_speeds[0], forwards_is_clockwise[0]);
                let v_b = rot_to_lin(motor_speeds[1], forwards_is_clockwise[1]);
                let v_c = rot_to_lin(motor_speeds[2], forwards_is_clockwise[2]);

                let v_term1 = ((v_a + v_b - 2.0 * v_c) / 3.0).powi(2);
                let v_term2 = (v_a - v_b).powi(2) / 3.0;
                let v = f32::sqrt(v_term1 + v_term2);

                let a_term_top = f32::sqrt(3.0) * (v_a - v_b);
                let a_term_bot = v_a + v_b - 2.0 * v_c;
                let a = f32::atan2(a_term_top, a_term_bot) + FRAC_PI_6 + FRAC_PI_2;

                let w = (v_a + v_b + v_c) / 3.0;

                let v_x = v * a.cos();
                let v_y = v * a.sin();

                (Vector2::new(-v_y, -v_x), w / robot_radius)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn test_3(
        _name: &str,
        drive_system: DriveSystem<3>,
        velocity: Vector2<f32>,
        ang_velocity: f32,
        expected: [f32; 3],
    ) {
        let result = drive_system.get_motor_speed_omni(velocity, ang_velocity);

        expected.iter().enumerate().for_each(|(i, result)| {
            assert_relative_eq!(*result, expected[i], epsilon = 0.0001);
        });

        let result2 = drive_system.get_actual_vel_omni(result);
        assert_relative_eq!(result2.0.x, velocity.x, epsilon = 0.0001);
        assert_relative_eq!(result2.0.y, velocity.y, epsilon = 0.0001);
        assert_relative_eq!(result2.1, ang_velocity, epsilon = 0.0001);
    }

    fn test_n<const WHEELS: usize>(
        _name: &str,
        drive_system: DriveSystem<WHEELS>,
        velocity: Vector2<f32>,
        ang_velocity: f32,
        expected: [f32; WHEELS],
    ) {
        let result = drive_system.get_motor_speed_omni(velocity, ang_velocity);

        result.iter().enumerate().for_each(|(i, result)| {
            assert_relative_eq!(*result, expected[i], epsilon = 0.0001);
        });
    }

    #[test]
    fn test_4_omniwheel() {
        let omni_4 = DriveSystem::new_omniwheel(
            1.0,
            10.0,
            [
                Rotation2::new(0.0),
                Rotation2::new(1.0 * PI / 2.0),
                Rotation2::new(2.0 * PI / 2.0),
                Rotation2::new(3.0 * PI / 2.0),
            ],
            [true, true, true, true],
        )
        .expect("Failed to create drive system for test");

        for (name, vel, ang_vel, expected) in [
            ("right", Vector2::new(1.0, 0.0), 0.0, [0.0, -1.0, 0.0, 1.0]),
            ("up", Vector2::new(0.0, 1.0), 0.0, [1.0, 0.0, -1.0, 0.0]),
            ("left", Vector2::new(-1.0, 0.0), 0.0, [0.0, 1.0, 0.0, -1.0]),
            ("down", Vector2::new(0.0, -1.0), 0.0, [-1.0, 0.0, 1.0, 0.0]),
            ("45deg", Vector2::new(1.0, 1.0), 0.0, [1.0, -1.0, -1.0, 1.0]),
            ("spin", Vector2::new(0.0, 0.0), 1.0, [10.0; 4]),
            ("spin", Vector2::new(0.0, 0.0), -1.0, [-10.0; 4]),
            ("45+sp", Vector2::new(1.0, 1.0), 1.0, [11.0, 9.0, 9.0, 11.0]),
        ] {
            test_n(name, omni_4, vel, ang_vel, expected);
        }

        // drive system with 4th wheel reversed
        let back = DriveSystem::new_omniwheel(
            1.0,
            10.0,
            [
                Rotation2::new(0.0),
                Rotation2::new(1.0 * PI / 2.0),
                Rotation2::new(2.0 * PI / 2.0),
                Rotation2::new(3.0 * PI / 2.0),
            ],
            [true, true, true, false],
        )
        .expect("Failed to create drive system for test");

        for (name, vel, ang_vel, expected) in [
            ("back", Vector2::new(1.0, 0.0), 0.0, [0.0, -1.0, 0.0, -1.0]),
            ("back", Vector2::new(0.0, 1.0), 0.0, [1.0, 0.0, -1.0, 0.0]),
        ] {
            test_n(name, back, vel, ang_vel, expected);
        }
    }

    #[test]
    fn test_3_omniwheel() {
        let omni_3 = DriveSystem::new_omniwheel(
            1.0,
            10.0,
            [
                Rotation2::new(0.0),
                Rotation2::new(2.0 * PI / 3.0),
                Rotation2::new(4.0 * PI / 3.0),
            ],
            [true, true, true],
        )
        .expect("Failed to create drive system for test");

        let sr3_2: f32 = 3.0f32.sqrt() / 2.0;

        for (name, vel, ang_vel, expected) in [
            ("right", Vector2::new(1.0, 0.0), 0.0, [0.0, -sr3_2, sr3_2]),
            ("up", Vector2::new(0.0, 1.0), 0.0, [1.0, -0.5, -0.5]),
            ("left", Vector2::new(-1.0, 0.0), 0.0, [0.0, sr3_2, -sr3_2]),
            ("down", Vector2::new(0.0, -1.0), 0.0, [-1.0, 0.5, 0.5]),
            ("spin", Vector2::new(0.0, 0.0), 1.0, [0.1, 0.1, 0.1]),
            ("spin", Vector2::new(0.0, 0.0), -1.0, [-0.1, -0.1, -0.1]),
            (
                "r+sp",
                Vector2::new(1.0, 0.0),
                1.0,
                [0.1, -sr3_2 + 0.1, sr3_2 + 0.1],
            ),
        ] {
            test_3(name, omni_3, vel, ang_vel, expected);
        }
    }
}
