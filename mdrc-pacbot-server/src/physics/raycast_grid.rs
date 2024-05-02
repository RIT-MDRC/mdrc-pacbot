use array_init::array_init;
use rapier2d::geometry::Ray;

use crate::grid::{ComputedGrid, IntLocation, GRID_COLS, GRID_ROWS};

pub struct RaycastGrid {
    rows: [u32; GRID_ROWS],
}

impl RaycastGrid {
    /// Creates a new grid for raycasting on the physical grid that corresponds to the given
    /// logical grid.
    pub fn new(logical_grid: &ComputedGrid) -> Self {
        assert_eq!(GRID_ROWS, 32);
        assert_eq!(GRID_COLS, 32);

        let is_wall = |row: i8, col: i8| {
            [
                IntLocation::new(row, col),
                IntLocation::new(row + 1, col),
                IntLocation::new(row, col + 1),
                IntLocation::new(row + 1, col + 1),
            ]
            .into_iter()
            .all(|part| logical_grid.wall_at(&part))
        };

        Self {
            rows: array_init(|row| {
                let row = row.try_into().unwrap();
                let mut row_mask = 0;
                for col in 0..32 {
                    if is_wall(row, col) {
                        row_mask |= 1 << col;
                    }
                }
                row_mask
            }),
        }
    }

    /// Returns whether there is a wall at the given (physical grid) coordinates.
    /// If the coordinates are out of bounds, returns true.
    pub fn is_wall(&self, row: i8, col: i8) -> bool {
        if (0..32).contains(&row) && (0..32).contains(&col) {
            self.rows[row as usize] & (1 << col) != 0
        } else {
            true
        }
    }

    /// Returns whether the given (physical grid) coordinates are within a wall.
    /// If the coordinates are on a boundary or corner of a wall, the return value is unspecified.
    /// If the coordinates are out of bounds, returns true.
    #[allow(dead_code)]
    pub fn is_point_in_wall(&self, row: f32, col: f32) -> bool {
        self.is_wall(row.floor() as i8, col.floor() as i8)
    }

    /// Returns whether the circle at with the given (physical grid) center and radius intersects a
    /// wall.
    #[allow(dead_code)]
    pub fn is_circle_in_wall(&self, center_row: f32, center_col: f32, radius: f32) -> bool {
        debug_assert!(radius >= 0.0);

        // Get an iterator over the nearby solid wall tiles that could potentially intersect.
        let row_min = (center_row - radius).floor() as i8;
        let row_max = ((center_row + radius).ceil() - 1.0) as i8;
        let col_min = (center_col - radius).floor() as i8;
        let col_max = ((center_col + radius).ceil() - 1.0) as i8;
        let mut nearby_walls = (row_min..=row_max)
            .flat_map(|row| (col_min..=col_max).map(move |col| (row, col)))
            .filter(|&(row, col)| self.is_wall(row, col));

        nearby_walls.any(|(row, col)| {
            // Check if the circle intersects the wall square centered at (row+0.5, col+0.5).
            // Algorithm adapted from https://stackoverflow.com/a/402010.
            let wall_x = (row as f32) + 0.5;
            let wall_y = (col as f32) + 0.5;
            let x_dist = (center_row - wall_x).abs();
            let y_dist = (center_col - wall_y).abs();

            if x_dist > 0.5 + radius || y_dist > 0.5 + radius {
                return false;
            }

            if x_dist <= 0.5 || y_dist <= 0.5 {
                return true;
            }

            (x_dist - 0.5).powi(2) + (y_dist - 0.5).powi(2) <= radius.powi(2)
        })
    }

    /// Cast a ray and return
    ///  - the distance from the ray origin to the point where it intersects a wall, or
    ///  - `max_dist`,
    /// whichever is smaller.
    ///
    /// Assumes that `ray.dir` is unit length.
    pub fn raycast(&self, ray: Ray, max_dist: f32) -> f32 {
        debug_assert!(
            (1.0 - ray.dir.norm()).abs() < 1e-4,
            "raycast requries a normalized ray, but ray.dir.norm() = {}",
            ray.dir.norm()
        );

        let get_helpers = |pos: f32, dir: f32| {
            let i = pos.floor() as i8;
            let step = if dir > 0.0 { 1 } else { -1 };
            let dt = dir.abs().recip();

            let dist_to_edge = if dir > 0.0 {
                pos.floor() + 1.0 - pos
            } else {
                pos - pos.floor()
            };
            let next_t = dist_to_edge * dt;

            (i, next_t, step, dt)
        };
        let (mut x, mut x_next_t, x_step, x_dt) = get_helpers(ray.origin.x, ray.dir.x);
        let (mut y, mut y_next_t, y_step, y_dt) = get_helpers(ray.origin.y, ray.dir.y);

        if self.is_wall(x, y) {
            return 0.0;
        }

        loop {
            let hit_time;
            if x_next_t < y_next_t {
                hit_time = x_next_t;
                x_next_t += x_dt;
                x += x_step;
            } else {
                hit_time = y_next_t;
                y_next_t += y_dt;
                y += y_step;
            }

            if hit_time >= max_dist {
                return max_dist;
            }
            if self.is_wall(x, y) {
                return hit_time;
            }
        }
    }
}
