use crate::{
    grid::{computed_grid::ComputedGrid, standard_grid::StandardGrid},
    robot_definition::RobotDefinition,
};
use nalgebra::{Const, OPoint, Point2};

const CV_ERROR: f32 = 2.0;

pub fn estimate_location(
    _grid: StandardGrid,
    _cv_location: Option<Point2<i8>>,
    _distance_sensors: &[Result<Option<f32>, ()>; 4],
    robot: &RobotDefinition<3>,
) -> Option<Point2<f32>> {
    if let Some(cv_location) = _cv_location {
        let mut location = Point2::new(cv_location.x as f32, cv_location.y as f32);
        let computed_grid = _grid.compute_grid();

        let mut poses =
            get_estimated_poses(&computed_grid, cv_location, _distance_sensors, robot.radius);

        if let Some(right_pos) = poses[0] {
            if let Some(left_pos) = poses[2] {
                if get_dist(right_pos, location) > CV_ERROR
                    && get_dist(left_pos, location) > CV_ERROR
                {
                    let mut new_location = Point2::new(cv_location.x, cv_location.y);
                    if let Some(up_pos) = poses[1] {
                        if up_pos.y > location.y {
                            new_location.y += 1;
                        } else {
                            new_location.y -= 1;
                        }
                    } else if let Some(down_pos) = poses[3] {
                        if down_pos.y > location.y {
                            new_location.y += 1;
                        } else {
                            new_location.y -= 1;
                        }
                    }
                    poses = get_estimated_poses(
                        &computed_grid,
                        new_location,
                        _distance_sensors,
                        robot.radius,
                    )
                }
            } else if get_dist(right_pos, location) > CV_ERROR {
                let mut new_location = Point2::new(cv_location.x, cv_location.y);
                if let Some(up_pos) = poses[1] {
                    if up_pos.y > location.y {
                        new_location.y += 1;
                    } else {
                        new_location.y -= 1;
                    }
                } else if let Some(down_pos) = poses[3] {
                    if down_pos.y > location.y {
                        new_location.y += 1;
                    } else {
                        new_location.y -= 1;
                    }
                }
                poses = get_estimated_poses(
                    &computed_grid,
                    new_location,
                    _distance_sensors,
                    robot.radius,
                )
            }
        } else if let Some(left_pos) = poses[2] {
            if get_dist(left_pos, location) > CV_ERROR {
                let mut new_location = Point2::new(cv_location.x, cv_location.y);
                if let Some(up_pos) = poses[1] {
                    if up_pos.y > location.y {
                        new_location.y += 1;
                    } else {
                        new_location.y -= 1;
                    }
                } else if let Some(down_pos) = poses[3] {
                    if down_pos.y > location.y {
                        new_location.y += 1;
                    } else {
                        new_location.y -= 1;
                    }
                }
                poses = get_estimated_poses(
                    &computed_grid,
                    new_location,
                    _distance_sensors,
                    robot.radius,
                )
            }
        }

        if let Some(up_pos) = poses[1] {
            if let Some(down_pos) = poses[3] {
                if get_dist(up_pos, location) > CV_ERROR && get_dist(down_pos, location) > CV_ERROR
                {
                    let mut new_location = Point2::new(cv_location.x, cv_location.y);
                    if let Some(right_pos) = poses[0] {
                        if right_pos.x > location.x {
                            new_location.x += 1;
                        } else {
                            new_location.x -= 1;
                        }
                    } else if let Some(left_pos) = poses[2] {
                        if left_pos.x > location.x {
                            new_location.x += 1;
                        } else {
                            new_location.x -= 1;
                        }
                    }
                    poses = get_estimated_poses(
                        &computed_grid,
                        new_location,
                        _distance_sensors,
                        robot.radius,
                    )
                }
            } else if get_dist(up_pos, location) > CV_ERROR {
                let mut new_location = Point2::new(cv_location.x, cv_location.y);
                if let Some(right_pos) = poses[0] {
                    if right_pos.x > location.x {
                        new_location.x += 1;
                    } else {
                        new_location.x -= 1;
                    }
                } else if let Some(left_pos) = poses[2] {
                    if left_pos.x > location.x {
                        new_location.x += 1;
                    } else {
                        new_location.x -= 1;
                    }
                }
                poses = get_estimated_poses(
                    &computed_grid,
                    new_location,
                    _distance_sensors,
                    robot.radius,
                )
            }
        } else if let Some(down_pos) = poses[3] {
            if get_dist(down_pos, location) > CV_ERROR {
                let mut new_location = Point2::new(cv_location.x, cv_location.y);
                if let Some(right_pos) = poses[0] {
                    if right_pos.x > location.x {
                        new_location.x += 1;
                    } else {
                        new_location.x -= 1;
                    }
                } else if let Some(left_pos) = poses[2] {
                    if left_pos.x > location.x {
                        new_location.x += 1;
                    } else {
                        new_location.x -= 1;
                    }
                }
                poses = get_estimated_poses(
                    &computed_grid,
                    new_location,
                    _distance_sensors,
                    robot.radius,
                )
            }
        }

        if let Some(right_pos) = poses[0] {
            if let Some(left_pos) = poses[2] {
                if get_dist(right_pos, location) < get_dist(left_pos, location) {
                    location.x = right_pos.x;
                } else {
                    location.x = left_pos.x;
                }
            } else {
                location.x = right_pos.x;
            }
        } else if let Some(left_pos) = poses[2] {
            location.x = left_pos.x;
        }

        if let Some(up_pos) = poses[1] {
            if let Some(down_pos) = poses[3] {
                if get_dist(up_pos, location) < get_dist(down_pos, location) {
                    location.y = up_pos.y;
                } else {
                    location.y = down_pos.y;
                }
            } else {
                location.y = up_pos.y;
            }
        } else if let Some(down_pos) = poses[3] {
            location.y = down_pos.y;
        }

        Some(location)
    } else {
        None
    }
}

fn get_estimated_poses(
    grid: &ComputedGrid,
    cv_location: OPoint<i8, Const<2>>,
    distance_sensors: &[Result<Option<f32>, ()>; 4],
    radius: f32,
) -> [Option<Point2<f32>>; 4] {
    let cv_distances = get_ray_cast(cv_location, grid, radius);
    let cv_location = Point2::new(cv_location.x as f32, cv_location.y as f32);
    let mut points = [Some(cv_location); 4];

    for (i, distance_sensor) in distance_sensors.iter().enumerate() {
        let point = points[i].unwrap();
        match i {
            0 => {
                if let Ok(Some(dist)) = distance_sensor {
                    points[i] = Some(Point2::new(point.x + cv_distances[i] - dist, point.y));
                } else {
                    points[i] = None;
                }
            }
            1 => {
                if let Ok(Some(dist)) = distance_sensor {
                    points[i] = Some(Point2::new(point.x, point.y + cv_distances[i] - dist));
                } else {
                    points[i] = None;
                }
            }
            2 => {
                if let Ok(Some(dist)) = distance_sensor {
                    points[i] = Some(Point2::new(point.x - cv_distances[i] + dist, point.y));
                } else {
                    points[i] = None;
                }
            }
            3 => {
                if let Ok(Some(dist)) = distance_sensor {
                    points[i] = Some(Point2::new(point.x, point.y - cv_distances[i] + dist));
                } else {
                    points[i] = None;
                }
            }
            _ => (),
        }
    }

    points
}

fn get_ray_cast(loc: Point2<i8>, grid: &ComputedGrid, radius: f32) -> [f32; 4] {
    let mut distances = [0; 4];

    for (i, _) in distances.into_iter().enumerate() {
        if i % 2 == 0 {
            let dir: i8 = if i == 0 { 1 } else { -1 };
            let mut dist: i8 = 0;
            while !grid.wall_at(&Point2::new(loc.x + dist * dir, loc.y)) {
                dist += 1;
            }
            distances[i] = dist;
        } else {
            let dir: i8 = if i == 1 { 1 } else { -1 };
            let mut dist: i8 = 0;
            while !grid.wall_at(&Point2::new(loc.x, loc.y + dist * dir)) {
                dist += 1;
            }
            distances[i] = dist;
        }
    }

    let float_distances: [f32; 4] = distances
        .into_iter()
        .map(|x| x as f32 - radius)
        .collect::<Vec<f32>>()
        .try_into()
        .unwrap();

    float_distances
}

fn get_dist(p0: Point2<f32>, p1: Point2<f32>) -> f32 {
    let t0 = p1.x - p0.x;
    let t1 = p1.y - p0.y;
    (t0 * t0 + t1 * t1).sqrt()
}
