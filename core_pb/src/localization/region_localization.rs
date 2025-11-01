use crate::constants::GU_PER_M;
use crate::grid::standard_grid::{get_grid_regions, StandardGrid};
use crate::grid::Grid;
use crate::localization::cv_adjust::get_sim_ray_cast;
use crate::messages::MAX_SENSOR_ERR_LEN;
use crate::robot_definition::RobotDefinition;
#[cfg(feature = "micromath")]
use micromath::F32Ext;
use nalgebra::{Point2, Vector2};
use ordered_float::NotNan;

const VECTORS: [Vector2<i8>; 4] = [
    Vector2::new(1, 0),  // right
    Vector2::new(0, 1),  // up
    Vector2::new(-1, 0), // left
    Vector2::new(0, -1), // down
];

/// A [`Region`] of a [`Grid`] is an area where moving around the region yields continuous
/// theoretical distance sensor readings
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Region {
    pub low_xy: Point2<i8>,
    pub high_xy: Point2<i8>,

    pub dist_low_xy_to_wall: [i8; 4],
}

fn get_region_score(
    grid: StandardGrid,
    dists: [Result<Option<f32>, ()>; 4],
    robot_radius: f32,
    max_toi: f32,
    region: &Region,
) -> Option<(f32, Point2<f32>)> {
    // warn!("a");
    // get lesser sensor
    let smallest_x_sensor = [0, 2]
        .into_iter()
        .map(|i| (i, dists[i]))
        .flat_map(|(i, x)| x.ok().map(|x| (i, x.unwrap_or(max_toi))))
        .min_by_key(|(_, d)| NotNan::new(*d).unwrap());
    let smallest_y_sensor = [1, 3]
        .into_iter()
        .map(|i| (i, dists[i]))
        .flat_map(|(i, x)| x.ok().map(|x| (i, x.unwrap_or(max_toi))))
        .min_by_key(|(_, d)| NotNan::new(*d).unwrap());

    if smallest_x_sensor.is_none() || smallest_y_sensor.is_none() {
        // warn!("c");
        return None;
    }

    // combine sensor values using region expected values
    let mut est_p = Vector2::new(0.0, 0.0);
    for (i, d) in [smallest_x_sensor, smallest_y_sensor].into_iter().flatten() {
        let facing_dir = VECTORS[i].map(|x| x as f32);
        let predicted_edge_distance = f32::min(region.dist_low_xy_to_wall[i] as f32, max_toi);

        let predicted_edge_vector = facing_dir * predicted_edge_distance;
        let actual_vector = facing_dir * (d + robot_radius);

        // accumulate offset from region.low_xy
        // if the prediction matches actual, estimates at region.low_xy
        // if both are positive, indicating +x or +y facing direction:
        //   and prediction is higher (normal), we are more +x
        //   and actual is higher (rare), we are more -x (outside region)
        // if both are negative, indicating -x or -y facing direction:
        //   and actual is less negative (normal), we are more +x
        //   and prediction is less negative (rare), we are more -x (outside region)
        est_p += predicted_edge_vector - actual_vector;
    }

    let p = est_p + Vector2::new(region.low_xy.x as f32, region.low_xy.y as f32);
    // the estimation loses points for being outside the region
    let mut score = 0.0;
    if p.x < region.low_xy.x as f32 {
        score += region.low_xy.x as f32 - p.x;
    }
    if p.x > region.high_xy.x as f32 {
        score += p.x - region.high_xy.x as f32;
    }
    if p.y < region.low_xy.y as f32 {
        score += region.low_xy.y as f32 - p.y;
    }
    if p.y > region.high_xy.y as f32 {
        score += p.y - region.high_xy.y as f32;
    }
    let tol = 1.0;
    if p.x < region.low_xy.x as f32 - tol
        || p.x > region.high_xy.x as f32 + tol
        || p.y < region.low_xy.y as f32 - tol
        || p.y > region.high_xy.y as f32 + tol
    {
        return None;
    }
    // or if any sensors are reading larger than predicted
    for (i, d) in dists
        .iter()
        .enumerate()
        .flat_map(|(i, d)| d.ok().map(|d| (i, d.unwrap_or(max_toi))))
    {
        let max_possible_dist = match i {
            0 | 1 => region.dist_low_xy_to_wall[i],
            2 => region.dist_low_xy_to_wall[2] + (region.high_xy.x - region.low_xy.x),
            3 => region.dist_low_xy_to_wall[3] + (region.high_xy.y - region.low_xy.y),
            _ => unreachable!(),
        } as f32;
        if d > max_possible_dist + 0.5 {
            score += d - max_possible_dist;
        }
    }
    // look at the four grid locations surrounding this point
    let rounded_loc = (p.x.floor() as i8, p.y.floor() as i8);
    // warn!("b");
    for grid_loc in [
        rounded_loc,
        (rounded_loc.0 + 1, rounded_loc.1),
        (rounded_loc.0, rounded_loc.1 + 1),
        (rounded_loc.0 + 1, rounded_loc.1 + 1),
    ] {
        // strongly discourage estimating our location inside a wall
        if get_at(grid.get_grid(), Vector2::new(grid_loc.0, grid_loc.1))
            && (grid_loc.0 as f32 - p.x).powi(2) + (grid_loc.1 as f32 - p.y).powi(2)
                < robot_radius.powi(2) * 0.9
        {
            return None;
            // score += 2.0;
        }
    }
    Some((-score, Point2::new(p.x, p.y)))
}

fn get_at(grid: Grid, at: Vector2<i8>) -> bool {
    if at.x < 0 || at.y < 0 || at.x as usize >= grid.len() || at.y as usize >= grid[0].len() {
        true
    } else {
        grid[at.x as usize][at.y as usize]
    }
}

/// RegionLocalization
pub fn estimate_location_2(
    grid: StandardGrid,
    cv_location: Option<Point2<i8>>,
    distance_sensors: &[Result<Option<f32>, heapless::String<MAX_SENSOR_ERR_LEN>>; 4],
    robot: &RobotDefinition<3>,
    do_cv_adjust: bool,
) -> Option<Point2<f32>> {
    let mut dists = [Err(()); 4];
    for (i, d) in distance_sensors.iter().enumerate() {
        dists[i] = match d {
            Err(_) => Err(()),
            Ok(None) => Ok(None),
            Ok(Some(d)) => Ok(Some(*d)),
        };
    }
    estimate_location(
        grid,
        cv_location,
        dists,
        robot.radius,
        robot.sensor_distance * GU_PER_M,
        do_cv_adjust,
    )
    .or(cv_location.map(|p| p.map(|a| a as f32)))
}

pub fn is_close_to_box(
    low_xy: Point2<i8>,
    high_xy: Point2<i8>,
    p: Point2<i8>,
    tolerance: i8,
) -> bool {
    // let y = i8::min((p.y - high_xy.y).abs(), (p.y - low_xy.y).abs());
    // let x = i8::min((p.x - high_xy.x).abs(), (p.x - low_xy.x).abs());
    //
    // i8::min(x, y) <= tolerance
    low_xy.x - tolerance <= p.x
        && p.x <= high_xy.x + tolerance
        && low_xy.y - tolerance <= p.y
        && p.y <= high_xy.y + tolerance
}

#[allow(unused)]
pub fn estimate_location(
    grid: StandardGrid,
    mut cv_location: Option<Point2<i8>>,
    distance_sensors: [Result<Option<f32>, ()>; 4],
    robot_radius: f32,
    max_toi: f32,
    do_cv_adjust: bool,
) -> Option<Point2<f32>> {
    let mut best_p: Option<Point2<f32>> = None;
    let mut best_score = f32::MIN;

    if cv_location
        .map(|cv| {
            cv.x < 0
                || cv.x >= 32
                || cv.y < 0
                || cv.y >= 32
                || grid.get_grid()[cv.x.max(0) as usize][cv.y.max(0) as usize]
        })
        .unwrap_or(false)
    {
        cv_location = None;
    }
    // cv_location = Some(cv_location.unwrap_or(Point2::new(1, 1)));

    for region in get_grid_regions(grid) {
        if let Some(cv_location) = cv_location {
            if !is_close_to_box(region.low_xy, region.high_xy, cv_location, 1) {
                continue;
            }
        }

        if let Some((mut score, mut pos)) =
            get_region_score(grid, distance_sensors, robot_radius, max_toi, region)
        {
            if let Some(cv_location) = cv_location {
                score = score
                    - (pos.x - cv_location.x as f32).abs()
                    - (pos.y - cv_location.y as f32).abs();
            }
            if let (Some(sm_x), Some(cv_loc), true) = (
                [0, 2]
                    .into_iter()
                    .map(|i| (i, distance_sensors[i]))
                    .flat_map(|(i, x)| x.ok().map(|x| (i, x.unwrap_or(max_toi))))
                    .min_by_key(|(_, d)| NotNan::new(*d).unwrap()),
                cv_location,
                do_cv_adjust,
            ) {
                if sm_x.1 > 5.0 {
                    pos.x = cv_loc.x as f32;
                    // info!(
                    //     "{pos:?} {} {}",
                    //     pos.x.max(0.0).round(),
                    //     pos.y.max(0.0).round()
                    // );
                    if grid.get_grid()[pos.x.max(0.0).round() as usize]
                        [pos.y.max(0.0).round() as usize]
                    {
                        pos = cv_loc.map(|x| x as f32);
                    }
                }
            }
            if let (Some(sm_y), Some(cv_loc), true) = (
                [1, 3]
                    .into_iter()
                    .map(|i| (i, distance_sensors[i]))
                    .flat_map(|(i, x)| x.ok().map(|x| (i, x.unwrap_or(max_toi))))
                    .min_by_key(|(_, d)| NotNan::new(*d).unwrap()),
                cv_location,
                do_cv_adjust,
            ) {
                if sm_y.1 > 5.0 {
                    pos.y = cv_loc.y as f32;
                    // info!(
                    //     "{pos:?} {} {}",
                    //     pos.x.max(0.0).round(),
                    //     pos.y.max(0.0).round()
                    // );
                    if grid.get_grid()[pos.x.max(0.0).round() as usize]
                        [pos.y.max(0.0).round() as usize]
                    {
                        pos = cv_loc.map(|x| x as f32);
                    }
                }
            }
            // info!("{region:?} {pos:?}");
            if score > best_score {
                best_score = score;
                best_p = Some(pos);
            }
        }
    }
    if let Some(best_p) = best_p {
        Some(best_p)
    } else {
        let mut loc = cv_location.unwrap_or_default().map(|x| x as f32);
        let min = distance_sensors
            .iter()
            .enumerate()
            .flat_map(|x| x.1.ok().map(|o| o.map(|o| (x.0, o))))
            .flatten()
            .min_by_key(|x| NotNan::try_from(x.1).unwrap());
        if let Some((i, val)) = min {
            if val < 1.0 {
                let raycasts =
                    get_sim_ray_cast(cv_location.unwrap_or_default(), &grid.get_grid(), 0.0);
                match i {
                    0 => loc.x += raycasts[0] - robot_radius - val,
                    1 => loc.y += raycasts[1] - robot_radius - val,
                    2 => loc.x -= raycasts[2] - robot_radius - val,
                    3 => loc.y -= raycasts[3] - robot_radius - val,
                    _ => {}
                }
            }
        }
        Some(loc)
    }
    // Some(best_p.unwrap_or(cv_location.unwrap_or_default().map(|x| x as f32)))
}

#[cfg(feature = "std")]
pub fn get_possible_regions(
    grid: StandardGrid,
    distance_sensors: [Result<Option<f32>, ()>; 4],
    max_toi: f32,
    robot_radius: f32,
) -> Vec<(Region, Point2<f32>)> {
    let mut regions = vec![];

    for region in get_grid_regions(grid) {
        if let Some((score, pos)) =
            get_region_score(grid, distance_sensors, robot_radius, max_toi, region)
        {
            if score == 0.0 {
                regions.push((*region, pos))
            }
        }
    }

    regions
}
