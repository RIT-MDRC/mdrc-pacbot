use crate::grid::{Grid, GRID_SIZE};
use nalgebra::{Point2, Vector2};

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

fn get_at(grid: Grid, at: Vector2<i8>) -> bool {
    if at.x < 0 || at.y < 0 || at.x as usize >= grid.len() || at.y as usize >= grid[0].len() {
        false
    } else {
        grid[at.x as usize][at.y as usize]
    }
}

fn v_to_p(v: Vector2<i8>) -> Point2<i8> {
    Point2::new(v.x, v.y)
}

fn p_to_v(p: Point2<i8>) -> Vector2<i8> {
    Vector2::new(p.x, p.y)
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
enum PointType {
    Wall,
    VerticalBoundary(bool),
    HorizontalBoundary(bool),
}

fn is_special(boundary: Option<PointType>) -> bool {
    match boundary {
        Some(PointType::Wall) => true,
        None => false,
        Some(PointType::HorizontalBoundary(b)) | Some(PointType::VerticalBoundary(b)) => !b,
    }
}

const ONE_X: Vector2<i8> = Vector2::new(1, 0);
const ONE_Y: Vector2<i8> = Vector2::new(0, 1);

fn get_boundary(grid: Grid, p: Vector2<i8>) -> Option<PointType> {
    // If the point is a wall, returns None
    if get_at(grid, p) {
        Some(PointType::Wall)
    }
    // If the point lies on a vertical region boundary
    else if (get_at(grid, p - ONE_Y) && !get_at(grid, p - ONE_Y - ONE_X))
        || (get_at(grid, p + ONE_Y) && !get_at(grid, p + ONE_Y - ONE_X))
    {
        Some(PointType::VerticalBoundary(true))
    } else if (get_at(grid, p - ONE_Y) && !get_at(grid, p - ONE_Y + ONE_X))
        || (get_at(grid, p + ONE_Y) && !get_at(grid, p + ONE_Y + ONE_X))
    {
        Some(PointType::HorizontalBoundary(false))
    }
    // If the point lies on a horizontal region boundary
    else if (get_at(grid, p - ONE_X) && !get_at(grid, p - ONE_X - ONE_Y))
        || (get_at(grid, p + ONE_X) && !get_at(grid, p + ONE_X - ONE_Y))
    {
        Some(PointType::HorizontalBoundary(true))
    } else if (get_at(grid, p - ONE_X) && !get_at(grid, p - ONE_X + ONE_Y))
        || (get_at(grid, p + ONE_X) && !get_at(grid, p + ONE_X + ONE_Y))
    {
        Some(PointType::HorizontalBoundary(false))
    } else {
        None
    }
}

fn build_horizontal_region(grid: Grid, p: Vector2<i8>) -> Region {
    let mut end = p + ONE_X;
    while get_boundary(grid, end).is_none() {
        end += ONE_X;
    }
    Region {
        low_xy: v_to_p(p - ONE_Y),
        high_xy: v_to_p(end + ONE_Y),

        dist_low_xy_to_wall: VECTORS.map(|v| get_empty_for(grid, p - ONE_Y, v)),
    }
}

fn build_vertical_region(grid: Grid, p: Vector2<i8>) -> Region {
    let mut end = p + ONE_Y;
    while get_boundary(grid, end).is_none() {
        end += ONE_Y;
    }
    Region {
        low_xy: v_to_p(p - ONE_X),
        high_xy: v_to_p(end + ONE_X),

        dist_low_xy_to_wall: VECTORS.map(|v| get_empty_for(grid, p - ONE_X, v)),
    }
}

/// Looks at the given point and returns up to 1 region
///
/// - If the point is a wall, returns None
/// - If the point lies entirely at the bottom left (-x,-y) of a region bounded below (-y) and to
/// the left (-x) by walls, returns the corresponding region
/// - If the point lies on a vertical region boundary, where the n-wide 2-tall region
/// lies to the right (+x), returns the corresponding region
/// - If the point lies on a horizontal region boundary, where the 2-wide n-tall region
/// lies above (+y), returns the corresponding region
pub fn get_region_for_unique_p(grid: Grid, at: Point2<i8>) -> Option<Region> {
    let p = Vector2::new(at.x, at.y);
    match get_boundary(grid, p) {
        Some(PointType::Wall)
        | Some(PointType::VerticalBoundary(false))
        | Some(PointType::HorizontalBoundary(false)) => None,
        None => {
            if is_special(get_boundary(grid, p - ONE_X))
                && is_special(get_boundary(grid, p - ONE_Y))
            {
                if get_boundary(grid, p + ONE_X).is_none() {
                    Some(build_horizontal_region(grid, p - ONE_X))
                } else if get_boundary(grid, p + ONE_Y).is_none() {
                    Some(build_vertical_region(grid, p - ONE_Y))
                } else {
                    // 2x2 region
                    Some(build_horizontal_region(grid, p - ONE_X))
                }
            } else {
                None
            }
        }
        Some(PointType::VerticalBoundary(true)) => Some(build_horizontal_region(grid, p)),
        Some(PointType::HorizontalBoundary(true)) => Some(build_vertical_region(grid, p)),
    }
}

/// Gets the region that p is in, or if it on a boundary, the upper/right one
///
/// If p is a wall, returns None
#[allow(unused)]
pub fn get_region_for_p(grid: Grid, mut at: Point2<i8>) -> Option<Region> {
    if get_at(grid, p_to_v(at)) {
        None
    } else {
        loop {
            if let Some(r) = get_region_for_unique_p(grid, at) {
                return Some(r);
            } else if !get_at(grid, p_to_v(at) - ONE_X) {
                at.x -= 1;
            } else if !get_at(grid, p_to_v(at) - ONE_Y) {
                at.y -= 1;
            } else {
                // this shouldn't happen
                crate::driving::info!("Missing region for point {} {}", at.x, at.y);
                return None;
            }
        }
    }
}

fn get_empty_for(grid: Grid, mut at: Vector2<i8>, dir: Vector2<i8>) -> i8 {
    let mut count = 0;
    while !get_at(grid, at) {
        at += dir;
        count += 1;
    }
    count
}

#[cfg(feature = "std")]
#[allow(unused)]
pub fn get_all_regions(grid: Grid) -> Vec<(Point2<i8>, Region)> {
    (0..GRID_SIZE)
        .flat_map(|x| {
            (0..GRID_SIZE).map(move |y| {
                get_region_for_unique_p(grid, Point2::new(x as i8, y as i8))
                    .map(|r| (Point2::new(x as i8, y as i8), r))
            })
        })
        .flatten()
        .collect()
}

fn get_region_score(
    dists: [Result<f32, ()>; 4],
    robot_radius: f32,
    max_toi: f32,
    region: Region,
) -> Option<(f32, Point2<f32>)> {
    let mut est_p = Vector2::new(0.0, 0.0);
    let mut num_contributors = 0;

    for (i, d) in dists
        .iter()
        .enumerate()
        .flat_map(|(i, d)| d.map(|d| (i, d)))
    {
        let facing_dir = VECTORS[i].map(|x| x as f32);
        let predicted_edge_distance = f32::max(region.dist_low_xy_to_wall[i] as f32, max_toi);

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
        num_contributors += 1;
    }

    if num_contributors == 0 {
        None
    } else {
        let p = est_p / num_contributors as f32
            + Vector2::new(region.low_xy.x as f32, region.low_xy.y as f32);
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
        Some((-score, Point2::new(p.x, p.y)))
    }
}

#[allow(unused)]
pub fn estimate_location(
    grid: Grid,
    cv_location: Point2<i8>,
    distance_sensors: [Result<f32, ()>; 4],
    robot_radius: f32,
    max_toi: f32,
) -> Option<Point2<f32>> {
    let mut best_p: Option<Point2<f32>> = None;
    let mut best_distance = f32::MAX;
    let mut is_zero_score = false;

    for x in 0..GRID_SIZE as i8 {
        for y in 0..GRID_SIZE as i8 {
            if let Some(region) = get_region_for_unique_p(grid, Point2::new(x, y)) {
                if let Some((score, pos)) =
                    get_region_score(distance_sensors, robot_radius, max_toi, region)
                {
                    if score == 0.0 && !is_zero_score {
                        is_zero_score = true;
                        best_distance = f32::MAX;
                    }
                    if is_zero_score || score == 0.0 {
                        println!("found!");
                        let dist = (pos.x - cv_location.x as f32).powi(2)
                            + (pos.y - cv_location.y as f32).powi(2);
                        if dist < best_distance {
                            best_p = Some(pos);
                            best_distance = dist;
                        }
                    } else {
                        if score < best_distance {
                            best_distance = score;
                            best_p = Some(pos);
                        }
                    }
                }
            }
        }
    }

    best_p
}

#[cfg(feature = "std")]
pub fn get_possible_regions(
    grid: Grid,
    distance_sensors: [Result<f32, ()>; 4],
    max_toi: f32,
    robot_radius: f32,
) -> Vec<Region> {
    let mut regions = vec![];

    for x in 0..GRID_SIZE as i8 {
        for y in 0..GRID_SIZE as i8 {
            if let Some(region) = get_region_for_unique_p(grid, Point2::new(x, y)) {
                if let Some((score, _)) =
                    get_region_score(distance_sensors, robot_radius, max_toi, region)
                {
                    if score == 0.0 {
                        regions.push(region)
                    }
                }
            }
        }
    }

    regions
}
