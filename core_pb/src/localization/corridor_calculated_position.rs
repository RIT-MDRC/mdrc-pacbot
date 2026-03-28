use nalgebra::{Point2, Vector2};

use crate::{
    grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN,
    robot_definition::RobotDefinition,
};

/// current_estimate must lie in or between previous_target and next_target.
/// The algorithm's performance is undefined otherwise
pub struct CorridorCalculatedPosition {
    previous_target: Point2<i8>,
    current_estimate: Point2<f32>,
    next_target: Point2<i8>,
}

enum PartialRegion {
    Farther,
    Middle,
    Closer,
}

type SensorUsefulness = Option<bool /* has discontinuity */>;

#[derive(Debug, PartialEq)]
struct RegionInfo {
    lateral: [SensorUsefulness; 2], /* left/right */
    transverse: [bool; 2],          /* front/back */
}

pub const PARTIAL_REGION_SIZE: f32 = 0.13;

impl CorridorCalculatedPosition {
    fn compute_region_info(
        &self,
        grid: &StandardGrid,
        partial: PartialRegion,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
    ) -> RegionInfo {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

        match partial {
            PartialRegion::Farther => {
                let forward_ray = grid.ray_cast(forward_ray, self.previous_target.clone());
                let backward_ray = grid.ray_cast(backward_ray, self.next_target.clone());
                let left_ray = if grid.ray_cast(left_ray, self.next_target.clone()) {
                    Some(true)
                } else {
                    None
                };
                let right_ray = if grid.ray_cast(right_ray, self.next_target.clone()) {
                    Some(true)
                } else {
                    None
                };

                RegionInfo {
                    lateral: [left_ray, right_ray],
                    transverse: [forward_ray, backward_ray],
                }
            }
            PartialRegion::Middle => {
                let forward_ray = grid.ray_cast(forward_ray, self.previous_target.clone());
                let backward_ray = grid.ray_cast(backward_ray, self.next_target.clone());

                let left_ray_prev = grid.wall_at(&(self.previous_target.clone() + left_ray));
                let left_ray_next = grid.wall_at(&(self.next_target.clone() + left_ray));

                let right_ray_prev = grid.wall_at(&(self.previous_target.clone() + right_ray));
                let right_ray_next = grid.wall_at(&(self.next_target.clone() + right_ray));

                let left_ray = Some(left_ray_next == left_ray_prev);
                let right_ray = Some(right_ray_next == right_ray_prev);

                RegionInfo {
                    lateral: [left_ray, right_ray],
                    transverse: [forward_ray, backward_ray],
                }
            }
            PartialRegion::Closer => {
                let forward_ray = grid.ray_cast(forward_ray, self.previous_target.clone());
                let backward_ray = grid.ray_cast(backward_ray, self.next_target.clone());
                let left_ray = if grid.ray_cast(left_ray, self.previous_target.clone()) {
                    Some(true)
                } else {
                    None
                };
                let right_ray = if grid.ray_cast(right_ray, self.previous_target.clone()) {
                    Some(true)
                } else {
                    None
                };

                RegionInfo {
                    lateral: [left_ray, right_ray],
                    transverse: [forward_ray, backward_ray],
                }
            }
        }
    }

    fn get_sensor_values(
        &self,
        x_length: &i8,
        y_length: &i8,
        distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
        info: &RegionInfo,
    ) -> ((Option<f32>, Option<f32>), (Option<f32>, Option<f32>)) {
        let sensor_values_adjusted = {
            /* forward, backward, left, right */
            if *y_length > 0 {
                (
                    &distance_sensors[0], // up
                    &distance_sensors[2], // down
                    &distance_sensors[1], // left
                    &distance_sensors[3], // right
                )
            } else if *y_length < 0 {
                (
                    &distance_sensors[2], // down
                    &distance_sensors[0], // up
                    &distance_sensors[1], // left
                    &distance_sensors[3], // right
                )
            } else if *x_length > 0 {
                (
                    &distance_sensors[3], // right
                    &distance_sensors[1], // left
                    &distance_sensors[0], // up
                    &distance_sensors[2], // down
                )
            } else {
                (
                    &distance_sensors[1], // left
                    &distance_sensors[3], // right
                    &distance_sensors[2], // down
                    &distance_sensors[0], // up
                )
            }
        };

        let lateral_sensors = {
            (
                match info.lateral[0] {
                    Some(useful) => {
                        if useful {
                            if let Ok(Some(left_sensor)) = sensor_values_adjusted.2 {
                                Some(left_sensor.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    None => None,
                },
                match info.lateral[1] {
                    Some(useful) => {
                        if useful {
                            if let Ok(Some(right_sensor)) = sensor_values_adjusted.3 {
                                Some(right_sensor.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    None => None,
                },
            )
        };

        let transverse_sensors = {
            (
                if info.transverse[0] {
                    if let Ok(Some(fwd_sensor)) = sensor_values_adjusted.0 {
                        Some(fwd_sensor.clone())
                    } else {
                        None
                    }
                } else {
                    None
                },
                if info.transverse[1] {
                    if let Ok(Some(back_sensor)) = sensor_values_adjusted.1 {
                        Some(back_sensor.clone())
                    } else {
                        None
                    }
                } else {
                    None
                },
            )
        };

        (lateral_sensors, transverse_sensors)
    }

    fn pos_from_sensors(
        &self,
        x_length: &i8,
        y_length: &i8,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
        lateral_sensors: (Option<f32>, Option<f32>),
        transverse_sensors: (Option<f32>, Option<f32>),
        grid: &StandardGrid,
        robot_definition: &RobotDefinition<3>,
        cv_location: Option<Point2<i8>>,
    ) -> Point2<f32> {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

        let left_pos = {
            if let Some(left_sensor) = lateral_sensors.0 {
                let dist_to_left_wall =
                    grid.ray_cast_distance(left_ray, self.previous_target.clone()) as f32;
                let wall_to_robot = (0.5 + left_sensor + robot_definition.radius) * {
                    if *y_length < 0 || *x_length < 0 {
                        -1.0
                    } else {
                        1.0
                    }
                };
                Some(dist_to_left_wall + wall_to_robot)
            } else {
                None
            }
        };

        let right_pos = {
            if let Some(right_sensor) = lateral_sensors.1 {
                let dist_to_right_wall =
                    grid.ray_cast_distance(right_ray, self.previous_target.clone()) as f32;
                let wall_to_robot = (0.5 + right_sensor + robot_definition.radius) * {
                    if *y_length < 0 || *x_length < 0 {
                        -1.0
                    } else {
                        1.0
                    }
                };
                Some(dist_to_right_wall - wall_to_robot)
            } else {
                None
            }
        };

        let lateral_pos = {
            if let Some(left_pos) = left_pos {
                if let Some(right_pos) = right_pos {
                    (left_pos + right_pos) / 2.0
                } else {
                    left_pos
                }
            } else if let Some(right_pos) = right_pos {
                right_pos
            } else if *y_length != 0 {
                self.current_estimate.y
            } else {
                self.current_estimate.x
            }
        };

        let fwd_pos = {
            if let Some(fwd_sensor) = transverse_sensors.0 {
                let dist_to_forward_wall =
                    grid.ray_cast_distance(forward_ray, self.previous_target.clone()) as f32 + 0.5;
                Some(dist_to_forward_wall + fwd_sensor + robot_definition.radius)
            } else {
                None
            }
        };

        let back_pos = {
            if let Some(back_sensor) = transverse_sensors.1 {
                let dist_to_backward_wall =
                    grid.ray_cast_distance(backward_ray, self.next_target.clone()) as f32 + 0.5;
                Some(dist_to_backward_wall + back_sensor + robot_definition.radius)
            } else {
                None
            }
        };

        let transverse_pos = {
            if let Some(fwd_pos) = fwd_pos {
                if let Some(back_pos) = back_pos {
                    (fwd_pos + back_pos) / 2.0
                } else {
                    fwd_pos
                }
            } else if let Some(back_pos) = back_pos {
                back_pos
            }
            /* TODO: what to do when no transverse? */
            else if let Some(cv_location) = cv_location {
                if *y_length != 0 {
                    cv_location.y as f32
                } else {
                    cv_location.x as f32
                }
            } else {
                if *y_length != 0 {
                    self.current_estimate.y as f32
                } else {
                    self.current_estimate.x as f32
                }
            }
        };

        if *y_length != 0 {
            Point2::new(lateral_pos, transverse_pos)
        } else {
            Point2::new(transverse_pos, lateral_pos)
        }
    }

    pub fn estimate_location(
        &mut self,
        grid: StandardGrid,
        cv_location: Option<Point2<i8>>,
        distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
        robot_definition: &RobotDefinition<3>,
    ) -> Option<Point2<f32>> {
        let x_length = self.next_target.x - self.previous_target.x;
        let y_length = self.next_target.y - self.previous_target.y;

        let rays = {
            if y_length > 0 {
                (
                    Vector2::new(0, 1),  // up
                    Vector2::new(0, -1), // down
                    Vector2::new(-1, 0), // left
                    Vector2::new(1, 0),  // right
                )
            } else if y_length < 0 {
                (
                    Vector2::new(0, -1), // down
                    Vector2::new(0, 1),  // up
                    Vector2::new(-1, 0), // left
                    Vector2::new(1, 0),  // right
                )
            } else if x_length > 0 {
                (
                    Vector2::new(1, 0),  // right
                    Vector2::new(-1, 0), // left
                    Vector2::new(0, 1),  // up
                    Vector2::new(0, -1), // down
                )
            } else {
                (
                    Vector2::new(-1, 0), // left
                    Vector2::new(1, 0),  // right
                    Vector2::new(0, -1), // down
                    Vector2::new(0, 1),  // up
                )
            }
        };

        let partial = {
            if x_length != 0 {
                let avg_x: f32 = (self.next_target.x as f32 + self.previous_target.x as f32) / 2.0;
                let x_diff = self.current_estimate.x - avg_x * x_length as f32;
                if x_diff > PARTIAL_REGION_SIZE {
                    PartialRegion::Farther
                } else if x_diff < -PARTIAL_REGION_SIZE {
                    PartialRegion::Closer
                } else {
                    PartialRegion::Middle
                }
            } else {
                let avg_y: f32 = (self.next_target.y as f32 + self.previous_target.y as f32) / 2.0;
                let y_diff = self.current_estimate.y - avg_y * y_length as f32;
                if y_diff > PARTIAL_REGION_SIZE {
                    PartialRegion::Farther
                } else if y_diff < -PARTIAL_REGION_SIZE {
                    PartialRegion::Closer
                } else {
                    PartialRegion::Middle
                }
            }
        };

        let info = self.compute_region_info(&grid, partial, &rays);

        println!("info: {:?}", info);
        println!("previous_target: {}", self.previous_target);
        println!("next_target: {}", self.next_target);
        println!("current_estimate: {}", self.current_estimate);

        let sensor_values = self.get_sensor_values(&x_length, &y_length, distance_sensors, &info);

        println!("sensor values: {:?}", sensor_values);

        self.current_estimate = self.pos_from_sensors(
            &x_length,
            &y_length,
            &rays,
            sensor_values.0,
            sensor_values.1,
            &grid,
            robot_definition,
            cv_location,
        );
        println!("new estimate: {}", self.current_estimate);

        return Some(self.current_estimate);
    }

    /// Path planner should set the next point on the path as a hint to the localizer.
    /// Assume provided next_point is on a traversable square
    pub fn set_next_point(&mut self, next_point: Point2<i8>) {
        let x_diff_cur = (self.previous_target.x - (self.current_estimate.x.round() as i8)).abs();
        let y_diff_cur = (self.previous_target.y - (self.current_estimate.y.round() as i8)).abs();
        let x_diff_next = (self.next_target.x - next_point.x).abs();
        let y_diff_next = (self.next_target.y - next_point.y).abs();

        if (x_diff_cur <= 1 && y_diff_cur == 0) || (y_diff_cur <= 1 && x_diff_cur == 0) {
            // if current_estimate is still within 1 grid unit of previous_target
            // next target should be modified
            self.next_target = next_point;
        } else if (x_diff_next <= 1 && y_diff_next == 0) || (y_diff_next <= 1 && x_diff_next == 0) {
            // if next_point is within 1 grid unit of next_target
            // shift next_target to previous target before assinging next_target
            self.previous_target = self.next_target;
            self.next_target = next_point;
        } else {
            // do nothing currently, but this means a next point not reachable by the current
            // position of the robot has been passed in
        }
    }

    /// An assumption needs to be made here about the initial starting position of the robot.
    /// For now, the assumption will be that it is at (20, 15) and going up
    /// TODO: make better assumptions about start
    pub fn new(initial_estimate: Point2<f32>, grid: &StandardGrid) -> CorridorCalculatedPosition {
        let rounded_estimate = Point2::new(
            initial_estimate.x.round() as i8,
            initial_estimate.y.round() as i8,
        );
        CorridorCalculatedPosition {
            previous_target: rounded_estimate,
            current_estimate: initial_estimate,
            next_target: {
                if !grid.wall_at(&Point2::new(rounded_estimate.x + 1, rounded_estimate.y)) {
                    Point2::new(rounded_estimate.x + 1, rounded_estimate.y)
                } else if !grid.wall_at(&Point2::new(rounded_estimate.x - 1, rounded_estimate.y)) {
                    Point2::new(rounded_estimate.x - 1, rounded_estimate.y)
                } else
                /* invariant: this one has to be valid because of the way that the grid is laidout */
                {
                    Point2::new(rounded_estimate.x, rounded_estimate.y + 1)
                }
            },
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_initial_region_info() {
        let grid = StandardGrid::Pacman;
        let pos = CorridorCalculatedPosition::new(Point2::new(20.0, 15.0), &grid);

        let rays = (
            Vector2::new(1, 0),  // right
            Vector2::new(-1, 0), // left
            Vector2::new(0, 1),  // up
            Vector2::new(0, -1), // down
        );

        let info = pos.compute_region_info(&grid, PartialRegion::Closer, &rays);

        assert_eq!(
            info,
            RegionInfo {
                lateral: [None, Some(true)],
                transverse: [true, true]
            }
        );
    }

    #[test]
    pub fn test_different_region_info() {
        let grid = StandardGrid::Pacman;
        let pos = CorridorCalculatedPosition::new(Point2::new(20.0, 15.0), &grid);

        let rays = (
            Vector2::new(1, 0),  // right
            Vector2::new(-1, 0), // left
            Vector2::new(0, 1),  // up
            Vector2::new(0, -1), // down
        );

        let info = pos.compute_region_info(&StandardGrid::Pacman, PartialRegion::Closer, &rays);

        assert_eq!(
            info,
            RegionInfo {
                lateral: [None, Some(true)],
                transverse: [true, true]
            }
        );
    }
}
