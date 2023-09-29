use rapier2d::na::{Point2, Vector2};

pub struct IMU {
    pub noise_std: f32,
}

pub struct OmniWheel {
    pub radius: f32,
}

pub struct Motor {
    pub wheel: OmniWheel,

    pub relative_position: Point2<f32>,
    pub forward_direction: Vector2<f32>,
}

pub struct DistanceSensor {
    pub relative_position: Point2<f32>,

    pub noise_std: f32,
    pub max_range: f32,
}

pub struct Robot {
    pub collider_radius: f32,
    pub density: f32,

    pub imu: Option<IMU>,
    pub motors: Vec<Motor>,
    pub distance_sensors: Vec<DistanceSensor>,
}
