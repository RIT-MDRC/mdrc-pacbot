use nalgebra::{Point2, Vector2};

use crate::{
    grid::{standard_grid::StandardGrid, Grid, GRID_SIZE},
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

// TODO
pub const MAX_SENSOR_DISTANCE: i8 = todo!();
pub const PARTIAL_REGION_SIZE: f32 = 0.13;

impl CorridorCalculatedPosition {
    fn wall_at(grid: &Grid, p: Point2<i8>) -> bool {
        if p.x >= GRID_SIZE as i8 || p.y >= GRID_SIZE as i8 || p.x < 0 || p.y < 0 {
            true
        } else {
            grid[p.x as usize][p.y as usize]
        }
    }

    fn ray_cast(dir: Vector2<i8>, loc: Point2<i8>, grid: &Grid) -> bool {
        let mut dist: i8 = 0;
        let mut p = loc;
        let dir = dir.map(|x| x as i8);

        while !Self::wall_at(grid, p) {
            if dist >= MAX_SENSOR_DISTANCE {
                return false;
            }

            p += dir;
            dist += 1;
        }

        true
    }

    fn compute_region_info(&self, grid: StandardGrid, partial: PartialRegion) -> RegionInfo {
        let x_length = self.next_target.x - self.previous_target.x;
        let y_length = self.next_target.y - self.previous_target.y;

        let (forward_ray, backward_ray, left_ray, right_ray) = {
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

        let grid = grid.get_grid();

        match partial {
            PartialRegion::Farther => {
                let forward_ray = Self::ray_cast(forward_ray, self.previous_target.clone(), &grid);
                let backward_ray = Self::ray_cast(backward_ray, self.next_target.clone(), &grid);
                let left_ray = if Self::ray_cast(left_ray, self.next_target.clone(), &grid) {
                    Some(false)
                } else {
                    None
                };
                let right_ray = if Self::ray_cast(right_ray, self.next_target.clone(), &grid) {
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
                let forward_ray = Self::ray_cast(forward_ray, self.previous_target.clone(), &grid);
                let backward_ray = Self::ray_cast(backward_ray, self.next_target.clone(), &grid);

                let left_ray_prev = Self::wall_at(&grid, self.previous_target.clone() + left_ray);
                let left_ray_next = Self::wall_at(&grid, self.next_target.clone() + left_ray);

                let right_ray_prev = Self::wall_at(&grid, self.previous_target.clone() + right_ray);
                let right_ray_next = Self::wall_at(&grid, self.next_target.clone() + right_ray);

                let left_ray = Some(left_ray_next == left_ray_prev);
                let right_ray = Some(right_ray_next == right_ray_prev);

                RegionInfo {
                    lateral: [left_ray, right_ray],
                    transverse: [forward_ray, backward_ray],
                }
            }
            PartialRegion::Closer => {
                let forward_ray = Self::ray_cast(forward_ray, self.previous_target.clone(), &grid);
                let backward_ray = Self::ray_cast(backward_ray, self.next_target.clone(), &grid);
                let left_ray = if Self::ray_cast(left_ray, self.previous_target.clone(), &grid) {
                    Some(false)
                } else {
                    None
                };
                let right_ray = if Self::ray_cast(right_ray, self.previous_target.clone(), &grid) {
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

    pub fn estimate_location(
        &mut self,
        grid: StandardGrid,
        cv_location: Option<Point2<i8>>,
        distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
        robot_definition: &RobotDefinition<3>,
    ) -> Option<Point2<f32>> {
        let x_length = self.next_target.x - self.previous_target.x;
        let y_length = self.next_target.y - self.previous_target.y;

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

        let info = self.compute_region_info(grid, partial);

        todo!();
    }
}
