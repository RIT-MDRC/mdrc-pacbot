//! Tracks the robot's position over time

use crate::grid::{ComputedGrid, Direction};
use crate::physics::{PacbotSimulation, GROUP_ROBOT, GROUP_WALL};
use crate::robot::{DistanceSensor, Robot};
use crate::util::stopwatch::Stopwatch;
use rand::rngs::ThreadRng;
use rand::Rng;
use rapier2d::na::{Isometry2, Point2, Vector2};
use rapier2d::prelude::{
    ColliderSet, InteractionGroups, QueryFilter, QueryPipeline, Ray, RigidBodySet, Rotation,
};
use rayon::prelude::*;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

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

/// Tracks the robot's position over time
pub struct ParticleFilter {
    /// Robot specifications
    robot: Robot,
    /// The grid used to find empty spaces; to change this, create a new particle filter
    grid: ComputedGrid,
    /// Guesses for the current location, ordered by measured accuracy
    points: Vec<Isometry2<f32>>,
    /// Cross-thread reference to the current distance sensor readings
    distance_sensors: Arc<Mutex<Vec<Option<f32>>>>,
    /// The current best guess
    best_guess: Isometry2<f32>,

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
        distance_sensors: Arc<Mutex<Vec<Option<f32>>>>,
        options: ParticleFilterOptions,
    ) -> Self {
        Self {
            points: Vec::new(),
            distance_sensors,
            grid,
            robot,
            best_guess: start,
            options,
        }
    }

    fn random_point_near(&self, point: Point2<u8>) -> Isometry2<f32> {
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
    fn random_point(&self) -> Isometry2<f32> {
        let mut rng = rand::thread_rng();

        let node = rng.gen_range(0..self.grid.walkable_nodes().len());
        let node = self.grid.walkable_nodes()[node];

        self.random_point_at(node, rng)
    }

    /// Generate a random valid point around a certain walkable square
    fn random_point_at(&self, node: Point2<u8>, mut rng: ThreadRng) -> Isometry2<f32> {
        // the central square (radius r) is where pacbot could be placed if there were walls all around
        let r = 1.0 - self.robot.collider_radius;

        // if r > 0.5, some of the cells are overlapping - cut off the edges of the central square
        if r >= 0.5 {
            let mut left_bottom = Point2::new(node.x as f32 - r, node.y as f32 - r);
            let mut right_top = Point2::new(node.x as f32 + r, node.y as f32 + r);

            if self.grid.next(&node, &Direction::Up).is_some() {
                right_top.y = node.y as f32 + 0.5;
            }
            if self.grid.next(&node, &Direction::Down).is_some() {
                left_bottom.y = node.y as f32 - 0.5;
            }
            if self.grid.next(&node, &Direction::Left).is_some() {
                left_bottom.x = node.x as f32 - 0.5;
            }
            if self.grid.next(&node, &Direction::Right).is_some() {
                right_top.x = node.x as f32 + 0.5;
            }

            let rand_x = rng.gen_range(left_bottom.x..right_top.x);
            let rand_y = rng.gen_range(left_bottom.y..right_top.y);

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
                        node.x as f32 + rng.gen_range(-r..r),
                        node.y as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                );
            }
            area_selector -= center_square_area;

            let direction = valid_directions[(area_selector / rectangle_area).floor() as usize];
            match direction {
                Direction::Up => Isometry2::new(
                    Vector2::new(
                        node.x as f32 + rng.gen_range(-r..r),
                        node.y as f32 + rng.gen_range(r..0.5),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Down => Isometry2::new(
                    Vector2::new(
                        node.x as f32 + rng.gen_range(-r..r),
                        node.y as f32 + rng.gen_range(-0.5..-r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Left => Isometry2::new(
                    Vector2::new(
                        node.x as f32 + rng.gen_range(-0.5..-r),
                        node.y as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
                Direction::Right => Isometry2::new(
                    Vector2::new(
                        node.x as f32 + rng.gen_range(r..0.5),
                        node.y as f32 + rng.gen_range(-r..r),
                    ),
                    rng.gen_range(0.0..2.0 * PI),
                ),
            }
        }
    }

    /// Update the particle filter, using the same rigid body set as the start
    pub fn update(
        &mut self,
        cv_position: Point2<u8>,
        rigid_body_set: &mut RigidBodySet,
        collider_set: &mut ColliderSet,
        query_pipeline: &QueryPipeline,
        stopwatch: &Arc<Mutex<Stopwatch>>,
    ) {
        stopwatch.lock().unwrap().start();

        // extend the points to the correct length
        while self.points.len() < self.options.points {
            let point = self.random_point();
            self.points.push(point);
        }

        stopwatch.lock().unwrap().mark_segment("Extend points");

        // cut off any extra points
        while self.points.len() > self.options.points {
            self.points.pop();
        }

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Cut off extra points");

        let elite_boundary = self.options.elite;
        let genetic_boundary = self.options.points - self.options.random - self.options.purge;
        let random_ish_boundary = self.options.points - self.options.random;

        let _elite_points = 0..elite_boundary;
        let genetic_points = elite_boundary..genetic_boundary;
        let random_near_cv_points = genetic_boundary..random_ish_boundary;
        let random_points = random_ish_boundary..self.options.points;

        // randomize the last 'random' points
        for i in random_points {
            let point = self.random_point();
            self.points[i] = point;
        }

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Randomize last points");

        // randomize the last 'purge' points near the given approximate location
        let results: Vec<_> = random_near_cv_points
            .clone()
            .into_par_iter()
            .map(|_| self.random_point_near(cv_position))
            .collect();
        self.points[random_near_cv_points].copy_from_slice(&results);
        // for i in random_near_cv_points {
        //     self.points[i] = self.random_point_near(cv_position);
        // }

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Randomize last points near cv location");

        let mut rng = rand::thread_rng();

        for i in genetic_points {
            // Generate a biased index based on the configured strength
            let mut weighted_index =
                (rng.gen::<f32>().powf(self.options.elitism_bias) * elite_boundary as f32) as usize;

            // Ensure the weighted_index does not exceed the last index
            weighted_index = weighted_index.min(self.options.points);

            // Retrieve the selected point and apply a mutation
            let point = self.points[weighted_index];
            let new_point = self.modify_point(point);

            // Replace the current point with the new one
            self.points[i] = new_point;
        }

        stopwatch.lock().unwrap().mark_segment("Genetic points");

        // randomize any points that are within a wall or out of bounds
        for i in 0..self.options.points {
            // if the virtual distance sensor reads 0, it is inside a wall
            if Self::distance_sensor_ray(
                self.points[i],
                self.robot.distance_sensors[0],
                rigid_body_set,
                collider_set,
                query_pipeline,
            )
            .abs()
                < 0.05
                || self.points[i].translation.x < 0.0
                || self.points[i].translation.y < 0.0
                || self.points[i].translation.x > 32.0
                || self.points[i].translation.y > 32.0
            {
                self.points[i] = self.random_point();
            }
        }

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Randomize out of bounds or in wall points");

        let robot = self.robot.to_owned();

        // Sort points
        let distance_sensors = self
            .distance_sensors
            .lock()
            .expect("Failed to acquire distance sensors lock!")
            .to_owned();
        if distance_sensors.len() != self.robot.distance_sensors.len() {
            println!("Uh oh! Particle filter found the wrong number of distance sensors. Unexpected behavior may occur.");
            return;
        }

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Lock distance sensors");

        // Calculate distance sensor errors
        // Calculate distance sensor errors and pair with points
        let mut paired_points_and_errors: Vec<(&Isometry2<f32>, f32)> = self
            .points
            .par_iter()
            .map(|p| {
                (
                    p,
                    Self::distance_sensor_diff(
                        &robot,
                        *p,
                        &distance_sensors,
                        rigid_body_set,
                        collider_set,
                        query_pipeline,
                    ),
                )
            })
            .collect();

        stopwatch
            .lock()
            .unwrap()
            .mark_segment("Calculate distance sensor errors");

        // Sort the paired vector based on the error values
        paired_points_and_errors
            .sort_unstable_by(|(_, error_a), (_, error_b)| error_a.total_cmp(error_b));

        // Extract the sorted points from the pairs
        self.points = paired_points_and_errors
            .into_iter()
            .map(|(point, _)| *point)
            .collect();

        stopwatch.lock().unwrap().mark_segment("Sort points");

        self.best_guess = self.points[0];
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
    pub fn points(&self, count: usize) -> Vec<Isometry2<f32>> {
        self.points
            .iter()
            .map(|p| p.to_owned())
            .take(count)
            .collect()
    }

    /// Get the best guess
    pub fn best_guess(&self) -> Isometry2<f32> {
        self.best_guess
    }
}

impl PacbotSimulation {
    /// Update the particle filter
    pub fn pf_update(&mut self, position: Point2<u8>, pf_stopwatch: &Arc<Mutex<Stopwatch>>) {
        self.particle_filter.update(
            position,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &self.query_pipeline,
            pf_stopwatch,
        );
    }
}
