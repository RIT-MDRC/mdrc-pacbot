//! Tracks the robot's position over time

use crate::grid::{ComputedGrid, Direction, IntLocation};
use crate::network::PacbotSensors;
use crate::physics::PacbotSimulation;
use crate::robot::{DistanceSensor, Robot};
use crate::util::stopwatch::Stopwatch;
use crate::UserSettings;
use num_traits::Zero;
use ordered_float::NotNan;
use rand::rngs::ThreadRng;
use rand::Rng;
use rand_distr::{Distribution, WeightedError};
use rapier2d::na::{Complex, Isometry2, Point2, UnitComplex, Vector2};
use rapier2d::prelude::{Ray, Rotation};
use rayon::prelude::*;
use std::f32::consts::PI;
use std::iter;

use super::raycast_grid::RaycastGrid;

/// Values that can be tweaked to improve the performance of the particle filter
pub struct ParticleFilterOptions {
    /// The total number of points tracked
    pub points: usize,
}

#[derive(Clone, Copy)]
pub struct FilterPoint {
    pub loc: Isometry2<f32>,
}

impl FilterPoint {
    pub fn new(loc: Isometry2<f32>) -> Self {
        Self { loc }
    }
}

/// Tracks the robot's position over time
pub struct ParticleFilter {
    /// Robot specifications
    robot: Robot,
    /// The grid used to find empty spaces; to change this, create a new particle filter
    grid: ComputedGrid,
    /// The data structure for performing raycasts on the physical grid.
    raycast_grid: RaycastGrid,
    /// Guesses for the current location, ordered by measured accuracy
    points: Vec<FilterPoint>,
    /// The current best guess
    best_guess: FilterPoint,

    /// Values that can be tweaked to improve the performance of the particle filter
    options: ParticleFilterOptions,
}

impl ParticleFilter {
    /// Create a ParticleFilter
    ///
    /// Start determines the location around which the filter will generate initial particles
    pub fn new(
        grid: ComputedGrid,
        robot: Robot,
        start: Isometry2<f32>,
        options: ParticleFilterOptions,
    ) -> Self {
        let start_point = FilterPoint::new(start);
        Self {
            points: vec![start_point],
            raycast_grid: RaycastGrid::new(&grid),
            grid,
            robot,
            best_guess: start_point,
            options,
        }
    }

    pub fn set_options(&mut self, particle_filter_options: ParticleFilterOptions) {
        self.options = particle_filter_options;
    }

    pub fn set_robot(&mut self, robot: Robot) {
        self.robot = robot;
    }

    pub fn raycast_grid(&self) -> &RaycastGrid {
        &self.raycast_grid
    }

    /// Generate a completely random walkable point
    fn random_point_uniform(&self) -> Isometry2<f32> {
        let mut rng = rand::thread_rng();

        let node = rng.gen_range(0..self.grid.walkable_nodes().len());
        let node = self.grid.walkable_nodes()[node];

        self.random_point_at(node, rng)
    }

    /// Generate a random valid point around a certain walkable square
    fn random_point_at(&self, node: IntLocation, mut rng: ThreadRng) -> Isometry2<f32> {
        // the central square (radius r) is where pacbot could be placed if there were walls all around
        let r = 1.0 - self.robot.collider_radius;

        // if r > 0.5, some of the cells are overlapping - cut off the edges of the central square
        if r >= 0.5 {
            let mut top_left = Point2::new(node.row as f32 - r, node.col as f32 - r);
            let mut bottom_right = Point2::new(node.row as f32 + r, node.col as f32 + r);

            if self.grid.next(&node, &Direction::Up).is_some() {
                top_left.x = node.row as f32 - 0.5;
            }
            if self.grid.next(&node, &Direction::Down).is_some() {
                bottom_right.x = node.row as f32 + 0.5;
            }
            if self.grid.next(&node, &Direction::Left).is_some() {
                top_left.y = node.col as f32 - 0.5;
            }
            if self.grid.next(&node, &Direction::Right).is_some() {
                bottom_right.y = node.col as f32 + 0.5;
            }

            let rand_x = rng.gen_range(top_left.x..bottom_right.x);
            let rand_y = rng.gen_range(top_left.y..bottom_right.y);

            Isometry2::new(Vector2::new(rand_x, rand_y), rng.gen_range(0.0..2.0 * PI))
        } else {
            // if r < 0.5, there are gaps between the regions - add rectangles to the sides

            // determine if the region should be extended to the left, right, top, bottom
            let mut valid_directions = Vec::new();
            for direction in &[
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                if self.grid.next(&node, direction).is_some() {
                    valid_directions.push(direction);
                }
            }

            // Define the areas of the rectangles and the central square
            let center_square_area = (2.0 * r) * (2.0 * r);
            let rectangle_area = (0.5 - r) * (2.0 * r);
            let total_area = center_square_area + valid_directions.len() as f32 * rectangle_area;

            // Generate a random number to select a region
            let mut area_selector = rng.gen_range(0.0..total_area);

            // Decide the region and generate coordinates within that region
            if area_selector < center_square_area {
                return Isometry2::new(
                    Vector2::new(
                        node.row as f32 + rng.gen_range(-r..r),
                        node.col as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                );
            }
            area_selector -= center_square_area;

            let direction = valid_directions[(area_selector / rectangle_area).floor() as usize];
            match direction {
                Direction::Up => Isometry2::new(
                    Vector2::new(
                        node.row as f32 + rng.gen_range(-0.5..-r),
                        node.col as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Down => Isometry2::new(
                    Vector2::new(
                        node.row as f32 + rng.gen_range(r..0.5),
                        node.col as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Left => Isometry2::new(
                    Vector2::new(
                        node.row as f32 + rng.gen_range(-r..r),
                        node.col as f32 + rng.gen_range(-0.5..-r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Right => Isometry2::new(
                    Vector2::new(
                        node.row as f32 + rng.gen_range(-r..r),
                        node.col as f32 + rng.gen_range(r..0.5),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
            }
        }
    }

    /// Update the particle filter, using the same rigid body set as the start
    pub fn update(
        &mut self,
        velocity: (Vector2<f32>, f32),
        dt: f32,
        stopwatch: &mut Stopwatch,
        sensors: &PacbotSensors,
        settings: &UserSettings,
    ) {
        stopwatch.start();

        let robot = self.robot.to_owned();

        let mut rng = rand::thread_rng();

        let noise_mag = settings.pf_simulated_translation_noise * velocity.0.norm()
            + settings.pf_simulated_rotation_noise * velocity.1.abs()
            + settings.pf_generic_noise;
        let noise_dist = rand_distr::Normal::new(0.0, noise_mag * dt.sqrt()).unwrap();
        let gen_noise_value = |rng: &mut ThreadRng| rng.sample(noise_dist);
        // multiply velocity by dt to get the distance moved
        let delta_x = velocity.0.x * dt;
        let delta_y = velocity.0.y * dt;
        let delta_theta = velocity.1 * dt;
        // for point in &mut self.points {
        for i in 0..self.points.len() {
            let point = &mut self.points[i];
            let delta_x = delta_x + gen_noise_value(&mut rng);
            let delta_y = delta_y + gen_noise_value(&mut rng);
            let delta_theta = delta_theta + gen_noise_value(&mut rng) * 1.0;
            let angle = point.loc.rotation.angle();
            let mut delta_x_rotated = delta_x * angle.cos() - delta_y * angle.sin();
            let mut delta_y_rotated = delta_x * angle.sin() + delta_y * angle.cos();

            // Raycast along the delta vector. If the point would have translated into a wall,
            // have it move a shorter distance that does not intersect (as much).
            let delta = Vector2::new(delta_x_rotated, delta_y_rotated);
            if delta.norm() > 1e-5 {
                let origin = point.loc.translation.vector.into();
                let dir = delta.normalize();
                let ray = Ray::new(origin, dir);
                let dist =
                    self.raycast_grid.raycast(ray, f32::INFINITY) - self.robot.collider_radius;
                if dist < delta.norm() {
                    [delta_x_rotated, delta_y_rotated] = (dir * dist).into();
                }
            }
            

            if rng.gen_bool(settings.pf_kidnapping_chance.into()) {
                self.points[i] = FilterPoint::new(self.random_point_uniform());

            } else {
                point.loc.translation.x += delta_x_rotated;
                point.loc.translation.y += delta_y_rotated;
                point.loc.rotation = Rotation::new(angle + delta_theta);
            }
        }

        stopwatch.mark_segment("Move each point by pacbot velocity + noise");

        // Get the sensor measurements.
        let actual_sensor_readings = sensors.distance_sensors.map(|x| Some(x as f32 / 88.9));
        if actual_sensor_readings.len() != self.robot.distance_sensors.len() {
            println!("Uh oh! Particle filter found the wrong number of distance sensors. Unexpected behavior may occur.");
            return;
        }

        stopwatch.mark_segment("Get distance sensors");

        // Compute the weight for each particle using the sensor measurement model.
        // First, compute the log-likelihoods.
        let mut point_weights: Vec<f32> = self
            .points
            .par_iter()
            .map(|p| {
                let [x, y] = p.loc.translation.into();
                if self.raycast_grid.is_in_wall(x, y) {
                    // This point is in a wall, so its likelihood is zero (log-likelihood = -inf).
                    f32::NEG_INFINITY
                } else {
                    // The log-likelihood is the negative sum of the sensor errors.
                    // This corresponds to modeling the sensor noise as independent Laplace
                    // distributions with scale parameter = 1.
                    // See: https://en.wikipedia.org/wiki/Laplace_distribution
                    -self.distance_sensor_diff(&robot, p.loc, &actual_sensor_readings)
                }
            })
            .collect();

        // Next, transform the log-likelihoods into (non-log) likelihood weights,
        // all scaled by a constant to avoid underflow from very small probabilities.
        if !point_weights.is_empty() {
            let mut max_log_likelihood = *point_weights
                .iter()
                .max_by_key(|&&w| NotNan::new(w).unwrap())
                .unwrap();
            if max_log_likelihood == f32::NEG_INFINITY {
                max_log_likelihood = 0.0; // Avoid computing (-inf) - (-inf) = NaN.
            }
            for w in &mut point_weights {
                *w = (*w - max_log_likelihood).exp();
            }
        }
        let point_weights = point_weights; // Make the weights immutable from this point forward.

        stopwatch.mark_segment("Calculate particle weights (scaled likelihoods)");

        // Set self.best_guess to the (weighted) mean particle.
        let mut total_weight = 0.0;
        let mut sum_pos = Vector2::zeros();
        let mut sum_dir = Complex::zero();
        for (point, &weight) in iter::zip(&self.points, &point_weights) {
            total_weight += weight;
            sum_pos += point.loc.translation.vector * weight;
            sum_dir += point.loc.rotation.into_inner() * weight;
        }
        if total_weight.is_finite() && total_weight > 0.0 {
            sum_pos /= total_weight;
            let sum_dir = UnitComplex::new_normalize(sum_dir);
            self.best_guess = FilterPoint::new(Isometry2::from_parts(sum_pos.into(), sum_dir));
        } else {
            eprintln!("Particle filter: total_weight={total_weight}, so not updating best_guess");
        }

        stopwatch.mark_segment("Compute mean point");

        // Resample particles using the likelihood weights.
        match rand_distr::WeightedAliasIndex::new(point_weights) {
            Ok(index_distribution) => {
                self.points = index_distribution
                    .sample_iter(&mut rng)
                    .take(self.points.len()) // TODO: should this immediately resample to n = self.options.points?
                    .map(|i| self.points[i])
                    .collect();
            }
            Err(WeightedError::NoItem | WeightedError::AllWeightsZero) => {
                // There are no particles with nonzero likelihood, so just skip resampling?
            }
            Err(err) => panic!("Failed to create WeightedAliasIndex: {err}"),
        }

        stopwatch.mark_segment("Resample points");

        // extend the points to the correct length
        while self.points.len() < self.options.points {
            // chance to uniformly add a random point or do one around an existing point
            let point = if rng.gen_bool(settings.pf_chance_near_other as f64)
                && !self.points.is_empty()
            {
                // grab random point to generate a point near. grab point from self.points
                let random_index = rng.gen_range(0..self.points.len());
                let chosen_point = self.points[random_index];
                // generate a new point some random distance around this chosen point by adding a random x, y and angle.
                let new_x = chosen_point.loc.translation.x
                    + rng.gen_range(-settings.pf_translation_limit..settings.pf_translation_limit);
                let new_y = chosen_point.loc.translation.y
                    + rng.gen_range(-settings.pf_translation_limit..settings.pf_translation_limit);
                let new_angle = chosen_point.loc.rotation.angle()
                    + rng.gen_range(-settings.pf_rotation_limit..settings.pf_rotation_limit);
                Isometry2::new(Vector2::new(new_x, new_y), new_angle)
            } else {
                self.random_point_uniform()
            };
            self.points.push(FilterPoint::new(point));
        }

        stopwatch.mark_segment("Add new points to fill up points list");

        // cut off any extra points
        self.points.truncate(self.options.points);

        stopwatch.mark_segment("Cut off extra points");
    }

    /// Given a location guess, measure the absolute difference against the real values
    fn distance_sensor_diff(
        &self,
        robot: &Robot,
        point: Isometry2<f32>,
        actual_values: &[Option<f32>],
    ) -> f32 {
        (0..actual_values.len())
            .map(|i| match actual_values[i] {
                None => 0.0,
                Some(x) => {
                    let sensor = robot.distance_sensors[i];

                    let toi = self.distance_sensor_ray(point, sensor);

                    let toi = (toi * 88.9).round() / 88.9;

                    (toi - x).abs()
                }
            })
            .sum()
    }

    /// Given a location guess, measure one sensor
    fn distance_sensor_ray(&self, point: Isometry2<f32>, sensor: DistanceSensor) -> f32 {
        let origin = point.transform_point(&sensor.relative_position);
        let dir = point.rotation * Rotation::new(sensor.relative_direction);
        let ray = Ray::new(origin, [dir.re, dir.im].into());

        self.raycast_grid.raycast(ray, sensor.max_range)
    }

    /// Get the best 'count' particle filter points
    pub fn points(&self, count: usize) -> Vec<FilterPoint> {
        self.points
            .iter()
            .map(|p| p.to_owned())
            .take(count)
            .collect()
    }

    /// Get the best guess
    pub fn best_guess(&self) -> FilterPoint {
        self.best_guess
    }
}

impl PacbotSimulation {
    /// Update the particle filter
    pub fn pf_update(
        &mut self,
        velocity: (Vector2<f32>, f32),
        dt: f32,
        pf_stopwatch: &mut Stopwatch,
        sensors: &PacbotSensors,
        settings: &UserSettings,
    ) {
        self.particle_filter
            .update(velocity, dt, pf_stopwatch, sensors, settings);
    }
}
