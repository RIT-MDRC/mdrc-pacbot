use crate::grid::{Grid, GRID_SIZE};
use crate::messages::MAX_SENSOR_ERR_LEN;
use crate::{grid::standard_grid::StandardGrid, robot_definition::RobotDefinition};
#[cfg(feature = "micromath")]
use nalgebra::ComplexField;
use nalgebra::{Point2, Vector2};
use ordered_float::NotNan;

// const CV_ERROR: f32 = 1.5;

const VECTORS: [Vector2<f32>; 4] = [
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 1.0),
    Vector2::new(-1.0, 0.0),
    Vector2::new(0.0, -1.0),
];

pub fn estimate_location(
    grid: StandardGrid,
    cv_location: Option<Point2<i8>>,
    distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
    robot: &RobotDefinition<3>,
    cv_error: f32,
) -> Option<Point2<f32>> {
    let cv_location_int = cv_location?;
    let cv_location_f32 = cv_location_int.map(|x| x as f32);

    let grid = grid.get_grid();
    let mut poses = get_estimated_poses(&grid, cv_location_int, distance_sensors, robot.radius);

    if [poses[0], poses[2]].iter().all(|x| {
        x.map(|pos| get_dist(pos, cv_location_f32) > cv_error)
            .unwrap_or(true)
    }) {
        let mut new_location = cv_location_int;
        if let Some(pos) = [poses[1], poses[3]].into_iter().flatten().next() {
            if pos.y > cv_location_f32.y {
                new_location.y += 1;
            } else {
                new_location.y -= 1;
            }
            poses = get_estimated_poses(&grid, new_location, distance_sensors, robot.radius);
        }
    }

    if [poses[1], poses[3]].iter().all(|x| {
        x.map(|pos| get_dist(pos, cv_location_f32) > cv_error)
            .unwrap_or(true)
    }) {
        let mut new_location = cv_location_int;
        if let Some(pos) = [poses[0], poses[2]].into_iter().flatten().next() {
            if pos.x > cv_location_f32.x {
                new_location.x += 1;
            } else {
                new_location.x -= 1;
            }
            poses = get_estimated_poses(&grid, new_location, distance_sensors, robot.radius);
        }
    }

    let x = [poses[0], poses[2]]
        .into_iter()
        .flatten()
        .min_by_key(|pos| {
            NotNan::new(get_dist(*pos, cv_location_f32)).unwrap_or(NotNan::new(0.0).unwrap())
        })
        .unwrap_or(cv_location_f32)
        .x;

    let y = [poses[1], poses[3]]
        .into_iter()
        .flatten()
        .min_by_key(|pos| {
            NotNan::new(get_dist(*pos, cv_location_f32)).unwrap_or(NotNan::new(0.0).unwrap())
        })
        .unwrap_or(cv_location_f32)
        .y;

    Some(Point2::new(x, y))
}

fn get_estimated_poses(
    grid: &Grid,
    cv_location: Point2<i8>,
    distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
    radius: f32,
) -> [Option<Point2<f32>>; 4] {
    let cv_distances = get_sim_ray_cast(cv_location, grid, radius);
    let cv_location = cv_location.map(|x| x as f32);

    [0, 1, 2, 3].map(|i| {
        distance_sensors[i]
            .clone()
            .ok()
            .flatten()
            .map(|dist| cv_location + (VECTORS[i] * (cv_distances[i] - dist)))
    })
}

fn get_sim_ray_cast(loc: Point2<i8>, grid: &Grid, radius: f32) -> [f32; 4] {
    VECTORS.map(|dir| {
        let mut dist: i8 = 0;
        let mut p = loc;
        let dir = dir.map(|x| x as i8);

        while !wall_at(grid, p) {
            p += dir;
            dist += 1;
        }

        dist as f32 - radius
    })
}

pub fn get_dist(p0: Point2<f32>, p1: Point2<f32>) -> f32 {
    let t0 = p1.x - p0.x;
    let t1 = p1.y - p0.y;
    (t0 * t0 + t1 * t1).sqrt()
}

fn wall_at(grid: &Grid, p: Point2<i8>) -> bool {
    if p.x >= GRID_SIZE as i8 || p.y >= GRID_SIZE as i8 || p.x < 0 || p.y < 0 {
        true
    } else {
        grid[p.x as usize][p.y as usize]
    }
}
