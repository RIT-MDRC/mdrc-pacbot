use nalgebra::{Point2, Vector2};

#[cfg(feature = "micromath")]
use micromath::F32Ext;

use crate::{
    grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN,
    robot_definition::RobotDefinition,
};

#[derive(Debug)]
pub struct Estimate<'a> {
    // - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
    // - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
    // - Ok(x) indicates an object x grid units in front of the sensor
    pub distance_sensors: [&'a Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],

    pub grid: &'a StandardGrid,
    pub robot_definition: &'a RobotDefinition<3>,
}

impl<'a> Estimate<'a> {
    /// Returns the amount, in positive (right) or negative (left) grid units, that Pacbot has deviated from the center line.
    /// If there are not two distance sensors that make sense that are available, this function should return None.
    /// TODO: In the future, we could potentially use a past run of this function to determine the lane drift of future positions.
    pub fn get_lane_drift(&mut self) -> Option<f32> {
        // Figure out: horizontal or vertical corridor? -> this should be saved to the estimate
        // - Based on this -> figure out the relevant sensors to use
        // Get distance reading from relevant sensors
        // If the distance is None, return -> can't be useful yet
        // If the distance is not None for both relevant sensors, calculate the difference between the two readings (right - left) and return it

        todo!()
    }
    
    pub fn estimate(
        &mut self
    ) -> Option<Point2<f32>> {

    }
}