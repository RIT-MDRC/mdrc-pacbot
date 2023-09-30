//! Describes the physical features of a Robot

use rapier2d::math::Rotation;
use rapier2d::na::{Point2, Vector2};
use std::f32::consts::PI;

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
    /// If this is (1, 0), then when the robot is at angle 0 and this motor drives forwards,
    /// a force in the +x direction will be applied to the robot
    pub forward_direction: Vector2<f32>,
}

/// Represents a Distance Sensor on a [`Robot`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceSensor {
    /// The position of the sensor relative to the [`Robot`]
    pub relative_position: Point2<f32>,
    /// The direction of the sensor relative to the [`Robot`]
    pub relative_direction: Rotation<f32>,

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
        let robot_radius = 0.45;

        for i in 0..8 {
            let angle = i as f32 * PI / 4.0;
            let rotation = Rotation::new(angle);

            distance_sensors.push(DistanceSensor {
                relative_position: rotation.transform_point(&Point2::new(robot_radius, 0.0)),
                relative_direction: rotation,

                noise_std: 0.0,
                max_range: 3.0,
            })
        }

        Self {
            collider_radius: robot_radius,
            density: 1.0,

            imu: None,
            motors: vec![],
            distance_sensors,
        }
    }
}
