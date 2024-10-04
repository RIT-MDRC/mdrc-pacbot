use crate::constants::MAX_ROBOT_PATH_LENGTH;
use crate::messages::SensorData;
use nalgebra::{Point2, Vector2};

pub fn pure_pursuit(
    _sensors: &SensorData,
    _path: &heapless::Vec<Point2<i8>, MAX_ROBOT_PATH_LENGTH>,
    _lookahead: f32,
) -> Option<Vector2<f32>> {
    None
}
