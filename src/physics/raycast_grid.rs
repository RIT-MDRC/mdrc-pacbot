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
    pub fn is_in_wall(&self, row: f32, col: f32) -> bool {
        self.is_wall(row.floor() as i8, col.floor() as i8)
    }

    /// Cast a ray and return
    ///  - the distance from the ray origin to the point where it intersects a wall, or
    ///  - `max_dist`,
    /// whichever is smaller.
    pub fn raycast(&self, ray: Ray, max_dist: f32) -> f32 {
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
