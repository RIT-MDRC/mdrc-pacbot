use defmt_or_log::macros::NoneError;
use nalgebra::{Point2, Vector2};

#[cfg(feature = "micromath")]
use micromath::F32Ext;
use pacbot_rs::location::{Direction, LocationState};

use crate::{
    constants::MAX_SENSOR_DISTANCE, grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN, robot_definition::RobotDefinition
};

#[derive(Debug)]
pub struct Estimate<'a> {
    // - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
    // - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
    // - Ok(x) indicates an object x grid units in front of the sensor
    pub distance_sensors: &'a [Option<f32>; 4],
    pub motor_speeds: [f32; 3],
    pub last_estimate: &'a LastEstimate,
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

    // // temporary, will figure out how to actually do the loop later
    // while (true) {
    //     if first cycle:
    //         old_data = None
    //         current_data = current_data
    //     else:
    //         old_data = current_data

    // }

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
        
    //  Returns an estimated coordinate location using the robot's velocity and timestamp from the last estimate
    pub fn get_coords_from_speed(&mut self) -> Option<Point2<f32>> {
        // self.motor_speeds is created in Estimator when the Estimate is created,
        // using sig_motor_speeds from shared robot data.
        let velocity: Option<Point2<f32>> = self.robot_definition.drive_system.get_actual_velocity(self.motor_speeds);
        let time_diff = SystemTime::now().duration_since(self.last_estimate.timestamp).ok()?.as_secs_f32();
        let distance: Option<Point2<f32>> = Point2::new(velocity[0] * time_diff,
                                                        velocity[1] * time_diff);
        
        
        return Point2::new(self.last_estimate.final_estimate[0] + distance[0], self.last_estimate.final_estimate[1] + distance[1]);
    }
    
    pub fn estimate(
        &mut self
    ) -> Option<Point2<f32>> {
        let distance_change: f32;
        //Later have a loop to store previous data
        let past_sensor: f32;
        //We need new sensor distance and old sensor distance
        if self.corridor_is_vertical() {
            //Cant find a var with current sensor
            //fix later or 
            let current_sensor = match self.distance_sensors[0] {
                Some(x) => x,
                None => {return None;}
            };

            distance_change = past_sensor - current_sensor;

            // previous grid position
            let (row, col) = LocationState::get_coords(&self);
            LocationState::new(row + distance_change, col, Direction::Up);
            
            //Update position

        }else{
            //Cant find a var with current sensor
            //fix later or 
            let current_sensor = match self.distance_sensors[2] {
                Some(x) => x,
                None => {return None;}
            };

            distance_change = past_sensor - current_sensor;

            // previous grid position
            let (row, col) = LocationState::get_coords(&self);
            LocationState::new(row, col + distance_change, Direction::Right);
            //Maybe??
            //LocationState::updatecoords(&mut self, (row, col + distance_change));
            
            //Update position
        }
        return None;
    }

    pub fn map_estimate(
        &mut self
    ) -> Option<Point2<f32>> {
        //Begin spot
        let begin_spot = self.grid.standard_grid.get_default_pacbot_isometry();// returns regular position
        
        //sensors movement spot

        //where end spot
    }
}