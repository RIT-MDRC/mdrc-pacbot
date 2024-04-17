//! Describes the physical features of a Robot

use rapier2d::math::Rotation;
use rapier2d::na::{Point2, Vector2};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_6, PI};

/// Represents an Inertial Measurement Unit, usually an accelerometer and gyroscope
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IMU {
    /// Standard deviation of the noise this sensor is expected to exhibit
    pub noise_std: f32,
}

/// Represents a single OmniWheel on a [`Motor`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmniWheel {
    /// Radius of the wheel
    pub radius: f32,
}

/// Represents a Motor on a [`Robot`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motor {
    /// The [`OmniWheel`] attached to this [`Motor`]
    pub wheel: OmniWheel,

    /// The position of the motor relative to the [`Robot`]
    pub relative_position: Point2<f32>,
    /// The forward direction of the motor relative to the [`Robot`]
    ///
    /// If this angle is 0, then when the robot is at angle 0 and this motor drives forwards,
    /// a force in the +x direction will be applied to the robot
    pub forward_direction: f32,
}

/// Represents a Distance Sensor on a [`Robot`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceSensor {
    /// The position of the sensor relative to the [`Robot`]
    pub relative_position: Point2<f32>,
    /// The (angle) direction of the sensor relative to the [`Robot`]
    pub relative_direction: f32,

    /// Standard deviation of the noise this sensor is expected to exhibit
    pub noise_std: f32,
    /// When the object is farther than this, the ray will be truncated to this distance
    pub max_range: f32,
}

/// Represents the physical features of a Robot
#[derive(Clone, Debug, PartialEq)]
pub struct Robot {
    /// The physical radius of the robot that should collide with walls
    pub collider_radius: f32,
    /// The average density of the robot
    pub density: f32,

    /// The IMU present on the robot
    pub imu: Option<IMU>,
    /// All motors attached to the robot
    pub motors: Vec<Motor>,
    /// All distance sensors on the robot
    pub distance_sensors: Vec<DistanceSensor>,
}

impl Default for Robot {
    fn default() -> Self {
        let mut distance_sensors = vec![];
        let robot_radius = 0.715;

        for i in 0..8 {
            let angle = i as f32 * PI / 4.0;
            let rotation = Rotation::new(angle);

            distance_sensors.push(DistanceSensor {
                relative_position: rotation.transform_point(&Point2::new(robot_radius, 0.0)),
                relative_direction: angle,

                noise_std: 0.0,
                max_range: 255.0 / 88.9,
            })
        }

        let circle_area = PI * robot_radius * robot_radius;
        Self {
            collider_radius: robot_radius,
            density: 1.0 / circle_area,

            imu: None,
            motors: vec![],
            distance_sensors,
        }
    }
}

/// Given the velocities of the wheels, find the velocity of the robot
pub fn wheel_velocities_to_robot_velocity(wheel_velocities: &[f32; 3]) -> (Vector2<f32>, f32) {
    let v_a = wheel_velocities[0];
    let v_b = wheel_velocities[1];
    let v_c = wheel_velocities[2];

    let v_term1 = ((v_a + v_b - 2.0 * v_c) / 3.0).powi(2);
    let v_term2 = (v_a - v_b).powi(2) / 3.0;
    let v = f32::sqrt(v_term1 + v_term2);

    let a_term_top = f32::sqrt(3.0) * (v_a - v_b);
    let a_term_bot = v_a + v_b - 2.0 * v_c;
    let a = f32::atan2(a_term_top, a_term_bot) + FRAC_PI_6 + FRAC_PI_2;

    let w = (v_a + v_b + v_c) / 3.0;

    let v_x = v * a.cos();
    let v_y = v * a.sin();

    (Vector2::new(-v_y, -v_x), w)
}
