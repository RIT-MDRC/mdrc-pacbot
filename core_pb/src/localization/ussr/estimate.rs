use nalgebra::{Point2, Vector2};

#[cfg(feature = "micromath")]
use micromath::F32Ext;

use crate::{
    constants::MAX_SENSOR_DISTANCE, grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN, robot_definition::RobotDefinition
};

#[derive(Debug)]
pub struct Estimate<'a> {
    // - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
    // - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
    // - Ok(x) indicates an object x grid units in front of the sensor
    pub distance_sensors: &'a [Option<f32>; 4],

    pub grid: &'a StandardGrid,
    pub robot_definition: &'a RobotDefinition<3>,
}

impl<'a> Estimate<'a> {
    pub fn corridor_is_vertical(&mut self) -> bool {
        let up_value = match self.distance_sensors[0] {
            Some(a) => a,
            None => MAX_SENSOR_DISTANCE
        };
        let down_value: f32 = match self.distance_sensors[1] {
            Some(a) => a,
            None => MAX_SENSOR_DISTANCE
        };
        let left_value: f32 = match self.distance_sensors[2] {
            Some(a) => a,
            None => MAX_SENSOR_DISTANCE
        };
        let right_value: f32 = match self.distance_sensors[3] {
            Some(a) => a,
            None => MAX_SENSOR_DISTANCE
        };
        return (up_value + down_value) > (left_value + right_value);
    }

    /// Returns the amount, in positive (right) or negative (left) grid units, that Pacbot has deviated from the center line.
    /// If there are not two distance sensors that make sense that are available, this function should return None.
    /// TODO: In the future, we could potentially use a past run of this function to determine the lane drift of future positions.
    pub fn get_lane_drift(&mut self) -> Option<f32> {
        // Figure out: horizontal or vertical corridor? -> this should be saved to the estimate
        // - Based on this -> figure out the relevant sensors to use
        // Get distance reading from relevant sensors
        
        let wall_distance;
        if self.corridor_is_vertical() {
            
            //returns None if One side is too far
            let left_sensor = match self.distance_sensors[2] {
                Some(x) => x,
                None => {return None;}
            };
            let right_sensor = match self.distance_sensors[3] {
                Some(x) => x,
                None => {return None;}
            };

            //calcs the distance difference bewteen the sensors
             wall_distance = right_sensor - left_sensor;
        } else {

            //returns None if One side is too far
            let up_sensor = match self.distance_sensors[0] {
                Some(x) => x,
                None => {return None;}
            };
            let down_sensor = match self.distance_sensors[1] {
                Some(x) => x,
                None => {return None;}
            };

            //calcs the distance difference bewteen the sensors
            wall_distance = up_sensor - down_sensor;
        }
        
        // If the distance is None, return -> can't be useful yet
        // If the distance is not None for both relevant sensors, calculate the difference between the two readings (right - left) and return it
        match wall_distance {
            0.0 => None,
            x => Some(x)
        }
    }
    
    pub fn estimate(
        &mut self
    ) -> Option<Point2<f32>> {

    }
}