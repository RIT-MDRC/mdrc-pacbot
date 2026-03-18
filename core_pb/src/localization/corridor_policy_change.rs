use nalgebra::{Point2, Vector2};

use crate::{
    constants::MAX_SENSOR_DISTANCE,
    grid::{computed_grid::ComputedGrid, standard_grid::StandardGrid, Grid, GRID_SIZE},
    messages::MAX_SENSOR_ERR_LEN,
    robot_definition::RobotDefinition,
};

pub struct CorridorCalculatedPosition {
    previous_target: Point2<i8>,
    current_estimate: Point2<f32>,
    next_target: Point2<i8>,
}

const VECTORS: [Vector2<i8>; 4] = [
    Vector2::new(1, 0),  // right
    Vector2::new(0, 1),  // up
    Vector2::new(-1, 0), // left
    Vector2::new(0, -1), // down
];

enum PartialRegion {
    Farther,
    Middle,
    Closer,
}

type SensorUsefulness = Option<bool /* has discontinuity */>;

struct RegionInfo {
    lateral: [SensorUsefulness; 2], /* left/right */
    transverse: [bool; 2],          /* front/back */
}

pub const PARTIAL_REGION_SIZE: f32 = 0.13;

impl CorridorCalculatedPosition {
    fn compute_region_info(
        &self,
        grid: &ComputedGrid,
        partial: PartialRegion,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
    ) -> RegionInfo {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

        match partial {
            PartialRegion::Farther => {
                let forward_ray = grid.ray_cast(forward_ray, self.previous_target.clone());
                let backward_ray = grid.ray_cast(backward_ray, self.next_target.clone());
                let left_ray = if grid.ray_cast(left_ray, self.next_target.clone()) {
                    Some(false)
                } else {
                    None
                };
                let right_ray = if grid.ray_cast(right_ray, self.next_target.clone()) {
                    Some(false)
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
                    Some(false)
                } else {
                    None
                };
                let right_ray = if grid.ray_cast(right_ray, self.previous_target.clone()) {
                    Some(false)
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
        grid: &ComputedGrid,
        rays: &(Vector2<i8>, Vector2<i8>, Vector2<i8>, Vector2<i8>),
        partial: &PartialRegion,
        robot_definition: &RobotDefinition<3>,
    ) {
        let (forward_ray, backward_ray, left_ray, right_ray) = rays;

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
                                Some(left_sensor)
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
                                Some(right_sensor)
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

        let mut lateral_pos: Option<f32> = None;

        if let (Some(left_sensor), Some(right_sensor)) = lateral_sensors {
            match partial {
                PartialRegion::Closer => {
                    let dist_to_left_wall =
                        grid.ray_cast_distance(left_ray, self.previous_target.clone()) as f32 + 0.5;
                    let dist_to_right_wall =
                        grid.ray_cast_distance(right_ray, self.previous_target.clone()) as f32
                            + 0.5;
                    lateral_pos = Some(
                        ((dist_to_left_wall + left_sensor + robot_definition.radius)
                            + (dist_to_right_wall + right_sensor + robot_definition.radius))
                            / 2.0,
                    );
                }
                _ => {
                    let dist_to_left_wall =
                        grid.ray_cast_distance(left_ray, self.next_target.clone()) as f32 + 0.5;
                    let dist_to_right_wall =
                        grid.ray_cast_distance(right_ray, self.next_target.clone()) as f32 + 0.5;
                    lateral_pos = Some(
                        ((dist_to_left_wall + left_sensor + robot_definition.radius)
                            + (dist_to_right_wall + right_sensor + robot_definition.radius))
                            / 2.0,
                    );
                }
            }
        } else if let Some(left_sensor) = lateral_sensors.0 {
            match partial {
                PartialRegion::Closer => {
                    let dist_to_left_wall =
                        grid.ray_cast_distance(left_ray, self.previous_target.clone()) as f32 + 0.5;
                    lateral_pos = Some(dist_to_left_wall + left_sensor + robot_definition.radius);
                }
                _ => {
                    let dist_to_left_wall =
                        grid.ray_cast_distance(left_ray, self.next_target.clone()) as f32 + 0.5;
                    lateral_pos = Some(dist_to_left_wall + left_sensor + robot_definition.radius);
                }
            }
        } else if let Some(right_sensor) = lateral_sensors.1 {
            match partial {
                PartialRegion::Closer => {
                    let dist_to_right_wall =
                        grid.ray_cast_distance(right_ray, self.previous_target.clone()) as f32
                            + 0.5;
                    lateral_pos = Some(dist_to_right_wall + right_sensor + robot_definition.radius);
                }
                _ => {
                    let dist_to_right_wall =
                        grid.ray_cast_distance(right_ray, self.next_target.clone()) as f32 + 0.5;
                    lateral_pos = Some(dist_to_right_wall + right_sensor + robot_definition.radius);
                }
            }
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

        let grid = grid.compute_grid();

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

        let grid = grid.grid.get_grid();

        let info = self.compute_region_info(&grid, partial, &rays);

        let values = self.get_sensor_values(
            &x_length,
            &y_length,
            distance_sensors,
            &info,
            &grid,
            &partial,
        );

        todo!();
    }
}
