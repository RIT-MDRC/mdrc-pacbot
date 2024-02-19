//! Tracks the robot's position over time

use crate::grid::{ComputedGrid, Direction, IntLocation};
use crate::network::PacbotSensors;
use crate::pathing::TargetVelocity;
use crate::physics::{PacbotSimulation, GROUP_ROBOT, GROUP_WALL};
use crate::robot::{DistanceSensor, Robot};
use crate::util::stopwatch::Stopwatch;
use native_dialog::Filter;
use rand::rngs::ThreadRng;
use rand::{random, Rng};
use rapier2d::math::Isometry;
use rapier2d::na::{Isometry2, Point2, Vector2};
use rapier2d::prelude::{
    ColliderSet, InteractionGroups, QueryFilter, QueryPipeline, Ray, RigidBodySet, Rotation,
};
use rayon::prelude::*;
use std::f32::consts::PI;

/// Values that can be tweaked to improve the performance of the particle filter
pub struct ParticleFilterOptions {
    /// The total number of points tracked
    ///
    /// Any points not included in elite, purge, or random will be moved slightly
    pub points: usize,

    /// The number of top guesses that are kept unchanged for the next generation
    pub elite: usize,
    /// The number of worst guesses that are deleted and randomly generated near the best guess
    pub purge: usize,
    /// The number of worst guesses that are deleted and randomly generated anywhere
    pub random: usize,

    /// Standard deviation of the distance from the current best known location
    pub spread: f32,
    /// 1 for no bias, greater than 1 for bias towards more elite points
    pub elitism_bias: f32,
    pub genetic_translation_limit: f32,
    pub genetic_rotation_limit: f32,
}

#[derive(Clone, Copy)]
pub struct FilterPoint {
    pub loc: Isometry2<f32>,
    time_alive: u16, // time alive in frames (has a maximum that should be taken into account)
}

impl FilterPoint {
    pub fn new(loc: Isometry2<f32>) -> Self {
        Self {
            loc,
            time_alive: 0,
        }
    }
    
    pub fn update_time_alive(&mut self) {
        self.time_alive = self.time_alive.saturating_add(1);
    }
}

/// Tracks the robot's position over time
pub struct ParticleFilter {
    /// Robot specifications
    robot: Robot,
    /// The grid used to find empty spaces; to change this, create a new particle filter
    grid: ComputedGrid,
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
            grid,
            robot,
            best_guess: start_point.clone(),
            options,
        }
    }

    pub fn set_options(&mut self, particle_filter_options: ParticleFilterOptions) {
        self.options = particle_filter_options;
    }

    pub fn set_robot(&mut self, robot: Robot) {
        self.robot = robot;
    }

    fn random_point_near(&self, point: IntLocation) -> Isometry2<f32> {
        let mut rng = rand::thread_rng();
        let distance = rng.gen_range(0.0..self.options.spread).floor() as usize;
        let mut node = point;
        for _ in 0..distance {
            let neighbors = self.grid.neighbors(&node);
            node = neighbors[rng.gen_range(0..neighbors.len())];
        }

        self.random_point_at(node, rng)
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
        velocity: Isometry2<f32>,
        dt: f32,
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        query_pipeline: &QueryPipeline,
        stopwatch: &mut Stopwatch,
        sensors: &PacbotSensors,
    ) {
        stopwatch.start();

        // retain points that are not inside walls
        self.points.retain(|&point| {
            // if the virtual distance sensor reads 0, it is inside a wall, so discard it
            !(Self::distance_sensor_ray(
                point.loc,
                DistanceSensor {
                    relative_position: Point2::new(0.0, 0.0),
                    relative_direction: 0.0,
                    noise_std: 0.0,
                    max_range: 1.0,
                },
                rigid_body_set,
                collider_set,
                query_pipeline,
            )
            .abs()
                < 0.05
                || point.loc.translation.x < 0.0
                || point.loc.translation.y < 0.0
                || point.loc.translation.x > 32.0
                || point.loc.translation.y > 32.0)
        });

        stopwatch.mark_segment("Remove invalid points inside walls");

        let robot = self.robot.to_owned();

        // TODO: go through each point and update it according to the velocity and angular velocity of the robot
        // multiply velocity by dt to get the distance moved
        let delta_x = velocity.translation.x * dt;
        let delta_y = velocity.translation.y * dt;
        let delta_theta = velocity.rotation.angle() * dt;
        for point in &mut self.points {
            point.loc.translation.x += delta_x;
            point.loc.translation.y += delta_y;
            point.loc.rotation = Rotation::new(point.loc.rotation.angle() + delta_theta);
            point.update_time_alive();
        }

        // Sort points
        let actual_sensor_readings: Vec<_> = sensors
            .distance_sensors
            .iter()
            .map(|x| Some(*x as f32 / 88.9))
            .collect();
        if actual_sensor_readings.len() != self.robot.distance_sensors.len() {
            println!("Uh oh! Particle filter found the wrong number of distance sensors. Unexpected behavior may occur.");
            return;
        }

        stopwatch.mark_segment("Lock distance sensors");

        // Calculate distance sensor errors
        // Calculate distance sensor errors and pair with points
        let mut paired_points_and_errors: Vec<(&FilterPoint, f32)> = self
            .points
            .par_iter()
            .map(|p| {
                (
                    p,
                    Self::distance_sensor_diff(
                        &robot,
                        (*p).loc,
                        &actual_sensor_readings,
                        rigid_body_set,
                        collider_set,
                        query_pipeline,
                    ),
                )
            })
            .collect();

        stopwatch.mark_segment("Calculate distance sensor errors");

        // go through every point and remove all that have an error greater than a certain threshold
        // TODO:_make tunable parameter
        paired_points_and_errors.retain(|(_, error)| *error < 2.0);
        stopwatch.mark_segment("Remove points with large error");

        // Sort the paired vector based on the error values
        paired_points_and_errors
            .sort_unstable_by(|(_, error_a), (_, error_b)| error_a.total_cmp(error_b));

        // Extract the sorted points from the pairs
        self.points = paired_points_and_errors
            .into_iter()
            .map(|(point, _)| *point)
            .collect();

        stopwatch.mark_segment("Sort points");

        // TODO: check that this is a good way to do this. Also move to a tunable parameter
        // Remove the last percentage of points
        // self.points
        //     .truncate((self.points.len() as f32 * 0.99) as usize);

        stopwatch.mark_segment("Remove least accurate points");

        // extend the points to the correct length since some have been pruned
        while self.points.len() < self.options.points {
            // chance to uniformly add a random point or do one around an existing point
            // TODO: make this a tunable parameter
            let point = if rand::thread_rng().gen_bool(0.99) && self.points.len() > 0 {
                // grab random point to generate a point near. grab point from self.points
                let random_index = rand::thread_rng().gen_range(0..self.points.len());
                let chosen_point = self.points[random_index];
                // generate a new point some random distance around this chosen point by adding a random x, y and angle. 
                // TODO: make this a tunable parameter, and mess with distribution of random spread
                let new_x = chosen_point.loc.translation.x + rand::thread_rng().gen_range(-0.3..0.3);
                let new_y = chosen_point.loc.translation.y + rand::thread_rng().gen_range(-0.3..0.3);
                let new_angle = chosen_point.loc.rotation.angle() + rand::thread_rng().gen_range(-0.3..0.3);
                Isometry2::new(Vector2::new(new_x, new_y), new_angle)
            } else {
                self.random_point_uniform()
            };
            self.points.push(FilterPoint::new(point));
        }

        stopwatch.mark_segment("Add new points to fill up points list");

        // cut off any extra points
        while self.points.len() > self.options.points {
            self.points.pop();
        }

        stopwatch.mark_segment("Cut off extra points");

        // TODO: this is slow
        // search for the point that has been around the longest
        self.best_guess = self.points.iter().max_by_key(|x| x.time_alive).unwrap().clone();
    }

    /// Given a location guess, measure the absolute difference against the real values
    fn distance_sensor_diff(
        robot: &Robot,
        point: Isometry2<f32>,
        actual_values: &Vec<Option<f32>>,
        rigid_body_set: &RigidBodySet,
        collider_set: &ColliderSet,
        query_pipeline: &QueryPipeline,
    ) -> f32 {
        (0..actual_values.len())
            .map(|i| match actual_values[i] {
                None => 0.0,
                Some(x) => {
                    let sensor = robot.distance_sensors[i];

                    let toi = Self::distance_sensor_ray(
                        point,
                        sensor,
                        rigid_body_set,
                        collider_set,
                        query_pipeline,
                    );

                    let toi = (toi * 88.9).round() / 88.9;

                    (toi - x).abs()
                }
            })
            .sum()
    }

    /// Given a location guess, measure one sensor
    fn distance_sensor_ray(
        point: Isometry2<f32>,
        sensor: DistanceSensor,
        rigid_body_set: &RigidBodySet,
        collider_set: &ColliderSet,
        query_pipeline: &QueryPipeline,
    ) -> f32 {
        let filter = QueryFilter::new().groups(InteractionGroups::new(
            GROUP_ROBOT.into(),
            GROUP_WALL.into(),
        ));

        let ray = Ray::new(
            point
                .translation
                .transform_point(&point.rotation.transform_point(&sensor.relative_position)),
            (point.rotation * Rotation::new(sensor.relative_direction))
                .transform_vector(&Vector2::new(1.0, 0.0)),
        );

        if let Some((_, toi)) = query_pipeline.cast_ray(
            rigid_body_set,
            collider_set,
            &ray,
            sensor.max_range,
            true,
            filter,
        ) {
            toi
        } else {
            sensor.max_range
        }
    }

    /// a small random translation and a small random rotation to the point
    fn modify_point(&mut self, point: Isometry2<f32>) -> Isometry2<f32> {
        let mut rng = rand::thread_rng();

        let translation_mutation_range =
            -self.options.genetic_translation_limit..self.options.genetic_translation_limit;
        let rotation_mutation_range =
            -self.options.genetic_rotation_limit..self.options.genetic_rotation_limit;

        let translation_mutation = Vector2::new(
            rng.gen_range(translation_mutation_range.clone()),
            rng.gen_range(translation_mutation_range),
        );

        let rotation_mutation = rng.gen_range(rotation_mutation_range);

        Isometry2::new(
            point.translation.vector + translation_mutation,
            point.rotation.angle() + rotation_mutation,
        )
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
        velocity: Isometry2<f32>,
        dt: f32,
        pf_stopwatch: &mut Stopwatch,
        sensors: &PacbotSensors,
    ) {
        self.particle_filter.update(
            velocity,
            dt,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &self.query_pipeline,
            pf_stopwatch,
            sensors,
        );
    }
}
