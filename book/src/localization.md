# Localization

The RIT pacbot team's localization requires 4 sensors, each facing in a cardinal direction (right, up, left
down). The sensors have a range that extends half the distance of the longest corridor of the grid.

## General Steps

1. First Pass Prediction
1. Second Pass Prediction
1. Combine Directional Information

## First Pass Prediction

The `cv_location` passed into the localizer function is a rough estimate of where pacbot is on the grid. To
get an initial estimate of where pacbot is based on sensor data, a raycast is performed in each of the
cardinal directions from the `cv_location`.

```
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
```

For the first prediction, an assumption is made: the walls that are hit by the actual sensors are the same
walls hit by the corresponding raycasts from `cv_location`. This assumption is not always true given the inaccuracy of `cv_location`, but can be fixed in the second pass prediction. Knowing this, each of the
sensors can give an estimate of pacbot's location on the corresponding axis by simply adding the raycast
vector and subtracting the sensor vector.

```
fn get_estimated_poses(
    grid: &Grid,
    cv_location: Point2<i8>,
    distance_sensors: &[Result<Option<f32>, ()>; 4],
    radius: f32,
) -> [Option<Point2<f32>>; 4] {
    let cv_distances = get_sim_ray_cast(cv_location, grid, radius);
    let cv_location = cv_location.map(|x| x as f32);

    [0, 1, 2, 3].map(|i| {
        distance_sensors[i]
            .ok()
            .flatten()
            .map(|dist| cv_location + (VECTORS[i] * (cv_distances[i] - dist)))
    })
}
```

Although the first prediction may be incorrect, the information from the estimated poses allows for
error correction.

## Second Pass Prediction

Once an estimated position is created from each of the sensors, each axis is checked to determine if it
actually hit the wall that was hit by the `cv_location` raycast. The check is done by observing if the
predicted position is different from `cv_location` by more than `CV_ERROR`. If the estimation is off
by more than `CV_ERROR` for both directions along an axis (right and left or up and down), then we must perform the second pass operation.

Before describing how the second pass works, it is important to understand that if one of the axis
has incorrect values for both sensors, then the other axis must have a correct value. This property
is a product of the grid, which requires pacbot to travel in a cardinal direction and also prevents it from
travelling diagonally through any corridors. Requiring at least one sensor to hit the same wall as
the `cv_location` raycast is essential for the second pass localization to correct the errors from
the first pass.

For the second pass, the program must find a position to raycast from in which the rays hit the same walls
that the sensors do. To do so, we check the position given by the axis that gives a position within `CV_ERROR`. If this position has a greater value along the axis as the `cv_location`, then we must raycast
again from one unit above the original `cv_location` on the corresponding axis. If the position has a lower value along the axis as the `cv_location`, then we must raycast again from one unit below the original `cv_location` on the corresponding axis. Once `new_location` is found, repeat the operations of the first
pass with `new_location` in place of `cv_location`. Since the assumption that was made is now true
(the rays hit the same walls as the sensors), the operation should give the correct position.

```
if [poses[0], poses[2]].iter().all(|x| {
        x.map(|pos| get_dist(pos, cv_location_f32) > CV_ERROR)
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
    x.map(|pos| get_dist(pos, cv_location_f32) > CV_ERROR)
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
```

## Combine Directional Information

The program integrates all of the sensor data by simply using one of the two sensors for each of the axis.
It prioritizes the values closest to `cv_location` and values that are not `None` or `Err()` for each axis.
If both sensors are not functional or do not detect a wall, that axis will remain whatever is given by
`cv_location`.

```
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
```