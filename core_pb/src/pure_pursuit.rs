use crate::messages::SensorData;
use crate::{constants::MAX_ROBOT_PATH_LENGTH, localization};
use localization::get_dist;
#[allow(unused_imports)]
use micromath::F32Ext;
use nalgebra::{Point2, Vector2};

const SPEED: f32 = 1.5;
const LOCAL_MAX_PATH_LENGTH: usize = MAX_ROBOT_PATH_LENGTH + 1;
const DIST_TOWARDS_CENTER: f32 = 0.3;

pub fn pure_pursuit(
    sensors: &SensorData,
    path: &heapless::Vec<Point2<i8>, MAX_ROBOT_PATH_LENGTH>,
    lookahead: f32,
) -> Option<Vector2<f32>> {
    let loc = sensors.location?;

    if path.is_empty() {
        let r = round_point(loc);
        if get_dist(loc, r) > DIST_TOWARDS_CENTER {
            return Some(get_vec(loc, r, false));
        } else {
            return None;
        }
    }

    let mut path_f32: heapless::Vec<Point2<f32>, LOCAL_MAX_PATH_LENGTH> =
        path.into_iter().map(|x| x.map(|y| y as f32)).collect();

    path_f32
        .insert(0, round_point(loc))
        .expect("CANNOT GET HERE");

    let closest_point = if path_f32.len() > 1 {
        get_closest_point(&loc, &path_f32, get_closest_segment(&loc, &path_f32))
    } else {
        loc
    };

    if let Some(pursuit_point) = get_pursuit_point(&closest_point, &path_f32, lookahead) {
        return Some(get_vec(loc, pursuit_point, true));
    }

    None
}

fn get_vec(loc: Point2<f32>, pursuit_point: Point2<f32>, normalize: bool) -> Vector2<f32> {
    let x = pursuit_point.x - loc.x;
    let y = pursuit_point.y - loc.y;
    let mag = (x * x + y * y).sqrt(); // could potentially do math here to ease acceleration as to not overshoot endpoint
    let mult = if normalize { SPEED / mag } else { 1.0 };
    Vector2::new(x * mult, y * mult)
}

fn get_pursuit_point(
    loc: &Point2<f32>,
    path: &heapless::Vec<Point2<f32>, LOCAL_MAX_PATH_LENGTH>,
    lookahead: f32,
) -> Option<Point2<f32>> {
    let mut intersections: heapless::Vec<Point2<f32>, LOCAL_MAX_PATH_LENGTH> = heapless::Vec::new();

    if path.len() == 1 {
        if let Some(intersection) = get_intersection(loc, *loc, path[0], lookahead) {
            intersections.push(intersection).expect("CANNOT GET HERE");
        }
    }

    for i in 0..(path.len() - 1) {
        if let Some(intersection) = get_intersection(loc, path[i], path[i + 1], lookahead) {
            if in_line(intersection, path[i], path[i + 1]) {
                intersections.push(intersection).expect("CANNOT GET HERE");
            }
        }
        if intersections.len() > 1 {
            break;
        }
    }

    if intersections.len() == 1 {
        return Some(intersections[0]);
    } else if intersections.len() == 2 {
        return Some(intersections[1]);
    }

    None
}

fn in_line(loc: Point2<f32>, p1: Point2<f32>, p2: Point2<f32>) -> bool {
    if p1.x == p2.x {
        loc.y >= p1.y && loc.y <= p2.y || loc.y <= p1.y && loc.y >= p2.y
    } else {
        loc.x >= p1.x && loc.x <= p2.x || loc.x <= p1.x && loc.x >= p2.x
    }
}

//gets intersection closest to p2
fn get_intersection(
    loc: &Point2<f32>,
    p1: Point2<f32>,
    p2: Point2<f32>,
    radius: f32,
) -> Option<Point2<f32>> {
    if p2.x - p1.x == 0.0 {
        let a: f32 = 1.0;
        let b = -2. * loc.y;
        let c = p1.x * p1.x - 2. * loc.x * p1.x + loc.x * loc.x + loc.y * loc.y - radius * radius;

        let q = b * b - 4. * a * c;

        if q < 0.0 {
            return None;
        }

        let y1 = (-b + q.sqrt()) / (2. * a);
        let y2 = (-b - q.sqrt()) / (2. * a);

        let i1 = Point2::new(p1.x, y1);
        let i2 = Point2::new(p1.x, y2);

        if get_dist(i1, p2) < get_dist(i2, p2) {
            return Some(i1);
        }
        return Some(i2);
    }

    let m = (p2.y - p1.y) / (p2.x - p1.x);
    let d = p1.y - m * p1.x;

    let a = 1. + m * m;
    let b = 2. * m * d - 2. * loc.x - 2. * loc.y * m;
    let c = d * d - radius * radius + loc.x * loc.x - 2. * loc.y * d + loc.y * loc.y;

    let q = b * b - 4. * a * c;

    if q < 0.0 {
        return None;
    }

    let x1 = (-b + q.sqrt()) / (2. * a);
    let x2 = (-b - q.sqrt()) / (2. * a);

    let y1 = m * x1 + d;
    let y2 = m * x2 + d;

    let i1 = Point2::new(x1, y1);
    let i2 = Point2::new(x2, y2);

    if get_dist(i1, p2) < get_dist(i2, p2) {
        return Some(i1);
    }
    Some(i2)
}

fn get_closest_point(
    loc: &Point2<f32>,
    path: &heapless::Vec<Point2<f32>, LOCAL_MAX_PATH_LENGTH>,
    i: usize,
) -> Point2<f32> {
    if i == path.len() - 1 {
        return path[path.len() - 1];
    }

    let p1 = path[i];
    let p2 = path[i + 1];

    let m: f32;
    let perp_m: f32;

    if p2.x - p1.x == 0.0 {
        return Point2::new(p1.x, loc.y);
    } else {
        m = (p2.y - p1.y) / (p2.x - p1.x);
        if m == 0.0 {
            return Point2::new(loc.x, p1.y);
        }
        perp_m = -1.0 / m;
    }

    let b = p1.y - m * p1.x;
    let perp_b = loc.y - perp_m * loc.x;

    let x = if b == perp_b {
        0.0
    } else {
        (m - perp_m) / (perp_b - b)
    };

    let y = perp_m * x + perp_b;

    Point2::new(x, y)
}

fn get_closest_segment(
    loc: &Point2<f32>,
    path: &heapless::Vec<Point2<f32>, LOCAL_MAX_PATH_LENGTH>,
) -> usize {
    if path.len() == 1 {
        return 0;
    }

    let (mut index1, mut dist1) = (0, get_dist(*loc, path[0]));

    for (i, point) in path.iter().enumerate() {
        let new_dist = get_dist(*loc, *point);
        if new_dist < dist1 {
            dist1 = new_dist;
            index1 = i;
        }
    }

    let (mut index2, mut dist2) = if index1 != 0 {
        (0, get_dist(*loc, path[0]))
    } else {
        (1, get_dist(*loc, path[1]))
    };

    for (i, point) in path.iter().enumerate() {
        let new_dist = get_dist(*loc, *point);
        if i != index1 && new_dist < dist2 {
            dist2 = new_dist;
            index2 = i;
        }
    }

    usize::min(index1, index2)
}

fn round_point(p: Point2<f32>) -> Point2<f32> {
    Point2::new((p.x + 0.5) as i8 as f32, (p.y + 0.5) as i8 as f32)
}
