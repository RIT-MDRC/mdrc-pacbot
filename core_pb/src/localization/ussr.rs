use nalgebra::{Point2, Vector2};

#[cfg(feature = "micromath")]
use micromath::F32Ext;

use crate::{
    grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN,
    robot_definition::RobotDefinition,
};

#[derive(Debug)]
pub struct Ussr {

}

impl Ussr {
    pub fn rotate_distance_sensors(distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4]) -> [&Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4] {
        return [
            &distance_sensors[0], // up
            &distance_sensors[1], // down
            &distance_sensors[2], // left
            &distance_sensors[3] // right
        ]
    }

    pub fn estimate_location(
        &mut self,
        grid: StandardGrid,
        raw_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
        robot_definition: &RobotDefinition<3>,
    ) -> Option<Point2<f32>> {
        // - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
        // - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
        // - Ok(x) indicates an object x grid units in front of the sensor
       let distance_sensors = Self::rotate_distance_sensors(distance_sensors);
    }

    /// An assumption needs to be made here about the initial starting position of the robot.
    /// For now, the assumption will be that it is at (20, 15) and going up
    /// TODO: make better assumptions about start
    pub fn new(initial_estimate: Point2<f32>, grid: &StandardGrid) -> Ussr {
        Ussr {
       
        }
    }
}