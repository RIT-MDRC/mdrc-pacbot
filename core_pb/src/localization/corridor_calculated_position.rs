use nalgebra::{Point2, Vector2};

#[cfg(feature = "micromath")]
use micromath::F32Ext;

use crate::{
    grid::standard_grid::StandardGrid, messages::MAX_SENSOR_ERR_LEN,
    robot_definition::RobotDefinition,
};

/// current_estimate must lie in or between previous_target and next_target.
/// The algorithm's performance is undefined otherwise
#[derive(Debug)]
pub struct CorridorCalculatedPosition {
    current_estimate: Point2<f32>,
    previous_target: Point2<i8>,
    next_target: Point2<i8>,
}

enum PartialRegion {
    Farther,
    Closer,
}

#[derive(Debug, PartialEq)]
struct RegionInfo {
    lateral: [bool; 2],    /* left/right */
    transverse: [bool; 2], /* front/back */
}

impl CorridorCalculatedPosition {
    fn compute_region_info(
        &self,
        grid: &StandardGrid,
        partial: PartialRegion,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
    ) -> RegionInfo {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

        let usable_target = match partial {
            PartialRegion::Farther => self.next_target.clone(),
            PartialRegion::Closer => self.previous_target.clone(),
        };

        let forward_usable = grid.ray_cast(forward_ray, self.previous_target.clone());
        let backward_usable = grid.ray_cast(backward_ray, self.next_target.clone());

        // left check
        let back_left = grid.wall_at(&Point2::new(
            usable_target.x + backward_ray.x + left_ray.x,
            usable_target.y + backward_ray.y + left_ray.y,
        ));
        let target_left = if grid.wall_at(&Point2::new(
            usable_target.x + left_ray.x,
            usable_target.y + left_ray.y,
        )) {
            true
        } else {
            grid.ray_cast(left_ray, usable_target)
        };
        let fwd_left = grid.wall_at(&Point2::new(
            usable_target.x + forward_ray.x + left_ray.x,
            usable_target.y + forward_ray.y + left_ray.y,
        ));

        let left_usable = back_left && target_left && fwd_left;

        // right check
        let back_right = grid.wall_at(&Point2::new(
            usable_target.x + backward_ray.x + right_ray.x,
            usable_target.y + backward_ray.y + right_ray.y,
        ));
        let target_right = if grid.wall_at(&Point2::new(
            usable_target.x + right_ray.x,
            usable_target.y + right_ray.y,
        )) {
            true
        } else {
            grid.ray_cast(right_ray, usable_target)
        };
        let fwd_right = grid.wall_at(&Point2::new(
            usable_target.x + forward_ray.x + right_ray.x,
            usable_target.y + forward_ray.y + right_ray.y,
        ));

        let right_usable = back_right && target_right && fwd_right;

        RegionInfo {
            lateral: [left_usable, right_usable],
            transverse: [forward_usable, backward_usable],
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
                    &distance_sensors[1], // up
                    &distance_sensors[3], // down
                    &distance_sensors[2], // left
                    &distance_sensors[0], // right
                )
            } else if *y_length < 0 {
                (
                    &distance_sensors[3], // down
                    &distance_sensors[1], // up
                    &distance_sensors[0], // right
                    &distance_sensors[2], // left
                )
            } else if *x_length > 0 {
                (
                    &distance_sensors[0], // right
                    &distance_sensors[2], // left
                    &distance_sensors[1], // up
                    &distance_sensors[3], // down
                )
            } else {
                (
                    &distance_sensors[2], // left
                    &distance_sensors[0], // right
                    &distance_sensors[3], // down
                    &distance_sensors[1], // up
                )
            }
        };

        // info!(
        //     "sensor values[adj]: fwd: {:?} back: {:?} left: {:?} right: {:?}, region_info: {:?}",
        //     sensor_values_adjusted.0,
        //     sensor_values_adjusted.1,
        //     sensor_values_adjusted.2,
        //     sensor_values_adjusted.3,
        //     info
        // );

        let lateral_sensors = {
            (
                if info.lateral[0] {
                    if let Ok(Some(left_sensor)) = sensor_values_adjusted.2 {
                        Some(left_sensor.clone())
                    } else {
                        None
                    }
                } else {
                    None
                },
                if info.lateral[1] {
                    if let Ok(Some(right_sensor)) = sensor_values_adjusted.3 {
                        Some(right_sensor.clone())
                    } else {
                        None
                    }
                } else {
                    None
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
        _x_length: &i8,
        y_length: &i8,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
        lateral_sensors: (Option<f32>, Option<f32>),
        transverse_sensors: (Option<f32>, Option<f32>),
        grid: &StandardGrid,
        robot_definition: &RobotDefinition<3>,
        cv_location: Option<Point2<i8>>,
        encoder_displacement: Option<Vector2<f32>>,
    ) -> Point2<f32> {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

        println!("encoder_displacement: {:?}", encoder_displacement);

        let partial = {
            let dist_to_prev = ((self.current_estimate.x - self.previous_target.x as f32).powi(2)
                + (self.current_estimate.y - self.previous_target.y as f32).powi(2))
            .sqrt();
            let dist_to_next = ((self.current_estimate.x - self.next_target.x as f32).powi(2)
                + (self.current_estimate.y - self.next_target.y as f32).powi(2))
            .sqrt();
            if dist_to_next < dist_to_prev {
                PartialRegion::Farther
            } else {
                PartialRegion::Closer
            }
        };

        let usable_target = match partial {
            PartialRegion::Farther => self.next_target.clone(),
            PartialRegion::Closer => self.previous_target.clone(),
        };

        let left_pos = {
            if let Some(left_sensor) = lateral_sensors.0 {
                let dist_to_left_wall =
                    grid.ray_cast_distance(left_ray, usable_target.clone()) as f32;
                let start_coord = if *y_length != 0 {
                    usable_target.x
                } else {
                    usable_target.y
                } as f32;
                let ray_dir = if *y_length != 0 {
                    left_ray.x
                } else {
                    left_ray.y
                } as f32;
                Some(
                    start_coord
                        + ray_dir * (dist_to_left_wall - left_sensor - robot_definition.radius),
                )
            } else {
                None
            }
        };

        let right_pos = {
            if let Some(right_sensor) = lateral_sensors.1 {
                let dist_to_right_wall =
                    grid.ray_cast_distance(right_ray, usable_target.clone()) as f32;
                let start_coord = if *y_length != 0 {
                    usable_target.x
                } else {
                    usable_target.y
                } as f32;
                let ray_dir = if *y_length != 0 {
                    right_ray.x
                } else {
                    right_ray.y
                } as f32;
                Some(
                    start_coord
                        + ray_dir * (dist_to_right_wall - right_sensor - robot_definition.radius),
                )
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
                self.current_estimate.x
            } else {
                self.current_estimate.y
            }
        };

        let fwd_pos = {
            if let Some(fwd_sensor) = transverse_sensors.0 {
                let dist_to_forward_wall =
                    grid.ray_cast_distance(forward_ray, self.previous_target.clone()) as f32;
                let start_coord = if *y_length != 0 {
                    self.previous_target.y
                } else {
                    self.previous_target.x
                } as f32;
                let ray_dir = if *y_length != 0 {
                    forward_ray.y
                } else {
                    forward_ray.x
                } as f32;
                Some(
                    start_coord
                        + ray_dir * (dist_to_forward_wall - fwd_sensor - robot_definition.radius),
                )
            } else {
                None
            }
        };

        let back_pos = {
            if let Some(back_sensor) = transverse_sensors.1 {
                let dist_to_backward_wall =
                    grid.ray_cast_distance(backward_ray, self.next_target.clone()) as f32;
                let start_coord = if *y_length != 0 {
                    self.next_target.y
                } else {
                    self.next_target.x
                } as f32;
                let ray_dir = if *y_length != 0 {
                    backward_ray.y
                } else {
                    backward_ray.x
                } as f32;
                Some(
                    start_coord
                        + ray_dir * (dist_to_backward_wall - back_sensor - robot_definition.radius),
                )
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
            // Dead reckoning: apply encoder displacement to previous estimate
            else if let Some(displacement) = encoder_displacement {
                let transverse_delta = if *y_length != 0 {
                    displacement.y
                } else {
                    displacement.x
                };
                if *y_length != 0 {
                    self.current_estimate.y + transverse_delta
                } else {
                    self.current_estimate.x + transverse_delta
                }
            } else if let Some(cv_location) = cv_location {
                // info!("CV!");
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
        encoder_displacement: Option<Vector2<f32>>,
    ) -> Option<Point2<f32>> {
        if let Some(cv) = cv_location {
            let cv_f = cv.cast::<f32>();
            let dist = ((self.current_estimate.x - cv_f.x).powi(2)
                + (self.current_estimate.y - cv_f.y).powi(2))
            .sqrt();
            if dist > 2.0 {
                self.current_estimate = cv_f;
                self.previous_target = cv;
                self.next_target = if !grid.wall_at(&Point2::new(cv.x + 1, cv.y)) {
                    Point2::new(cv.x + 1, cv.y)
                } else if !grid.wall_at(&Point2::new(cv.x - 1, cv.y)) {
                    Point2::new(cv.x - 1, cv.y)
                } else if !grid.wall_at(&Point2::new(cv.x, cv.y + 1)) {
                    Point2::new(cv.x, cv.y + 1)
                } else {
                    Point2::new(cv.x, cv.y - 1)
                };
            }
        }

        let x_length = self.next_target.x - self.previous_target.x;
        let y_length = self.next_target.y - self.previous_target.y;

        // info!("cv_location in estimate_location: {:?}", cv_location);

        // assume previous point is always the cv_location
        if let Some(cv_loc) = cv_location {
            self.previous_target = cv_loc;
        }

        let rays = {
            if y_length > 0 {
                (
                    Vector2::new(0, 1),  // forward (+y)
                    Vector2::new(0, -1), // backward (-y)
                    Vector2::new(-1, 0), // left (-x)
                    Vector2::new(1, 0),  // right (+x)
                )
            } else if y_length < 0 {
                (
                    Vector2::new(0, -1), // forward (-y)
                    Vector2::new(0, 1),  // backward (+y)
                    Vector2::new(1, 0),  // left (+x)
                    Vector2::new(-1, 0), // right (-x)
                )
            } else if x_length > 0 {
                (
                    Vector2::new(1, 0),  // forward (+x)
                    Vector2::new(-1, 0), // backward (-x)
                    Vector2::new(0, 1),  // left (+y)
                    Vector2::new(0, -1), // right (-y)
                )
            } else {
                (
                    Vector2::new(-1, 0), // forward (-x)
                    Vector2::new(1, 0),  // backward (+x)
                    Vector2::new(0, -1), // left (-y)
                    Vector2::new(0, 1),  // right (+y)
                )
            }
        };

        let partial = {
            let dist_to_prev = ((self.current_estimate.x - self.previous_target.x as f32).powi(2)
                + (self.current_estimate.y - self.previous_target.y as f32).powi(2))
            .sqrt();
            let dist_to_next = ((self.current_estimate.x - self.next_target.x as f32).powi(2)
                + (self.current_estimate.y - self.next_target.y as f32).powi(2))
            .sqrt();
            if dist_to_next < dist_to_prev {
                PartialRegion::Farther
            } else {
                PartialRegion::Closer
            }
        };

        let info = self.compute_region_info(&grid, partial, &rays);

        // info!("info: {:?}", info);
        // info!("current_estimate: {}", self.current_estimate);

        let sensor_values = self.get_sensor_values(&x_length, &y_length, distance_sensors, &info);

        // info!("sensor values: {:?}", sensor_values);

        self.current_estimate = self.pos_from_sensors(
            &x_length,
            &y_length,
            &rays,
            sensor_values.0,
            sensor_values.1,
            &grid,
            robot_definition,
            cv_location,
            encoder_displacement,
        );
        // info!("previous_target: {}", self.previous_target);
        // info!("next_target: {}", self.next_target);
        // info!("new estimate: {}", self.current_estimate);

        // invariant: current estimate cannot be outside of target ranges!
        // let rounded_estimate: Point2<i8> = Point2::new(
        //     self.current_estimate.x.round() as i8,
        //     self.current_estimate.y.round() as i8,
        // );
        let middle = Point2::new(
            (self.previous_target.x + self.next_target.x) / 2,
            (self.previous_target.y + self.next_target.y) / 2,
        );
        let diff = Point2::new(
            self.current_estimate.x - middle.x as f32,
            self.current_estimate.y - middle.y as f32,
        );
        let dist = (diff.x + diff.y).sqrt();
        // let diff_prev = rounded_estimate -
        let out_of_bounds = dist > 2.0_f32.sqrt();
        return if out_of_bounds {
            None
        } else {
            Some(self.current_estimate)
        };
    }

    /// Path planner should set the next point on the path as a hint to the localizer.
    /// Assume provided next_point is on a traversable square
    pub fn set_next_point(&mut self, next_path: &[Point2<i8>]) {
        for next_point in next_path {
            // let prev_point = Point2::new(self.previous_target.x, self.previous_target.y);
            let diff = next_point - self.previous_target;
            let magnitude = diff.x.abs() + diff.y.abs();
            if magnitude == 1 {
                self.next_target = *next_point;
            }
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

#[cfg(feature = "log")]
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_initial_region_info1() {
        let grid = StandardGrid::Pacman;
        let ccp = CorridorCalculatedPosition::new(Point2::new(20.0, 15.0), &grid);

        let rays = (
            Vector2::new(1, 0),  // right
            Vector2::new(-1, 0), // left
            Vector2::new(0, 1),  // up
            Vector2::new(0, -1), // down
        );

        let info = ccp.compute_region_info(&grid, PartialRegion::Closer, &rays);

        assert_eq!(
            info,
            RegionInfo {
                lateral: [false, true],
                transverse: [true, true]
            }
        );
    }

    #[test]
    pub fn test_region_info2() {
        let grid = StandardGrid::Pacman;
        let ccp = CorridorCalculatedPosition::new(Point2::new(1.0, 1.0), &grid);

        let rays = (
            Vector2::new(1, 0),  // right
            Vector2::new(-1, 0), // left
            Vector2::new(0, 1),  // up
            Vector2::new(0, -1), // down
        );

        let info = ccp.compute_region_info(&StandardGrid::Pacman, PartialRegion::Closer, &rays);

        assert_eq!(
            info,
            RegionInfo {
                lateral: [false, true],
                transverse: [true, true]
            }
        );
    }

    #[test]
    pub fn test_location_estimate_1_1_no_sensors() {
        let grid = StandardGrid::Pacman;
        let mut ccp = CorridorCalculatedPosition::new(Point2::new(1.0, 1.0), &grid);
        let robot = RobotDefinition::new(crate::names::RobotName::Stella);
        let cv_location = Some(Point2::new(1, 1));

        let estimated_location = ccp.estimate_location(
            grid,
            cv_location,
            &[Ok(None), Ok(None), Ok(None), Ok(None)],
            &robot,
            None,
        );

        // in the case where there are no sensors, it should just take CV_location to be the truth
        assert_eq!(
            estimated_location,
            cv_location.map(|p| Point2::new(p.x as f32, p.y as f32))
        );
    }

    #[test]
    pub fn test_invariant_violation_repro() {
        let grid = StandardGrid::Open;
        // At (20, 15), moving up to (20, 16)
        let mut ccp = CorridorCalculatedPosition {
            previous_target: Point2::new(20, 15),
            current_estimate: Point2::new(20.0, 15.0),
            next_target: Point2::new(20, 16),
        };
        let robot = RobotDefinition::new(crate::names::RobotName::Stella);

        // dummy values
        let sensor_up = 16.175;
        let sensor_left = 21.175;
        let sensor_down = 16.175;
        let sensor_right = 11.175;

        let sensors = [
            Ok(Some(sensor_up)),
            Ok(Some(sensor_left)),
            Ok(Some(sensor_down)),
            Ok(Some(sensor_right)),
        ];

        let result = ccp
            .estimate_location(grid, None, &sensors, &robot, None)
            .unwrap();

        // info!("Moving UP result: {:?}", result);
        assert!(
            result.x > 19.0 && result.x < 22.0,
            "X position {} is nonsense when moving UP!",
            result.x
        );
        assert!(
            result.y > 14.0 && result.y < 17.0,
            "Y position {} is nonsense when moving UP!",
            result.y
        );
    }

    #[test]
    pub fn test_moving_down_repro() {
        let grid = StandardGrid::Open;
        // At (20, 15), moving down to (20, 14)
        let mut ccp = CorridorCalculatedPosition {
            previous_target: Point2::new(20, 15),
            current_estimate: Point2::new(20.0, 15.0),
            next_target: Point2::new(20, 14),
        };
        let robot = RobotDefinition::new(crate::names::RobotName::Stella);

        let sensors = [
            Ok(Some(10.0)),
            Ok(Some(10.0)),
            Ok(Some(10.0)),
            Ok(Some(10.0)),
        ];

        let result = ccp
            .estimate_location(grid, None, &sensors, &robot, None)
            .unwrap();
        // info!("Moving DOWN result: {:?}", result);
        assert!(
            result.x > 19.0 && result.x < 21.0,
            "X position {} is nonsense when moving DOWN!",
            result.x
        );
    }
}
