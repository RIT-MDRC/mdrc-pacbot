use rapier2d::math::Rotation;
use rapier2d::na::{Point2, Vector2};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IMU {
    pub noise_std: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmniWheel {
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motor {
    pub wheel: OmniWheel,

    pub relative_position: Point2<f32>,
    pub forward_direction: Vector2<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DistanceSensor {
    pub relative_position: Point2<f32>,
    pub relative_direction: Rotation<f32>,

    pub noise_std: f32,
    pub max_range: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Robot {
    pub collider_radius: f32,
    pub density: f32,

    pub imu: Option<IMU>,
    pub motors: Vec<Motor>,
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
