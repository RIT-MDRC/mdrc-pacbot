//! Tracks the robot's position over time

use crate::constants::NUM_PARTICLE_FILTER_POINTS;
use crate::grid::{ComputedGrid, Direction};
use crate::physics::{PacbotSimulation, GROUP_ROBOT, GROUP_WALL};
use rand::rngs::ThreadRng;
use rand::Rng;
use rapier2d::na::{Isometry2, Point2, Vector2};
use rapier2d::prelude::{ColliderBuilder, ColliderHandle, InteractionGroups, RigidBodyBuilder};
use std::f32::consts::PI;

/// Tracks the robot's position over time
pub struct ParticleFilter {
    /// Guesses for the current location, ordered by measured accuracy
    points: Vec<ColliderHandle>,
    /// The current best guess
    best_guess: Isometry2<f32>,

    /// The number of top guesses that are kept unchanged for the next generation
    elite: usize,
    /// The number of worst guesses that are deleted and randomly generated near the best guess
    purge: usize,
    /// The number of worst guesses that are deleted and randomly generated anywhere
    random: usize,

    /// Standard deviation of the distance from the current best known location
    spread: f32,
}

impl ParticleFilter {
    /// Create a ParticleFilter
    pub fn new(elite: usize, purge: usize, random: usize, spread: f32) -> Self {
        Self {
            points: Vec::new(),
            best_guess: Isometry2::identity(),
            elite,
            purge,
            random,
            spread,
        }
    }

    /// Get the associated points
    pub fn points(&self) -> &Vec<ColliderHandle> {
        &self.points
    }
}

impl PacbotSimulation {
    /// Initialize the particle filter
    pub fn pf_initialize(
        &mut self,
        rng: &mut ThreadRng,
        grid: &ComputedGrid,
        start: Isometry2<f32>,
    ) {
        self.particle_filter.best_guess = start;
        self.particle_filter.points = Vec::new();

        let point_near = Point2::new(
            start.translation.vector.x.round() as u8,
            start.translation.vector.y.round() as u8,
        );

        let points = (0..NUM_PARTICLE_FILTER_POINTS)
            .map(|_| self.pf_random_point_near(rng, grid, point_near))
            .collect::<Vec<_>>();

        for point in points {
            let rigid_body = RigidBodyBuilder::dynamic().position(point).build();
            let rigid_body_handle = self.rigid_body_set.insert(rigid_body);

            let collider = ColliderBuilder::ball(self.robot_specifications.collider_radius)
                .density(self.robot_specifications.density)
                .collision_groups(InteractionGroups::new(
                    GROUP_ROBOT.into(),
                    GROUP_WALL.into(),
                )) // allows robots to only interact with walls, not other robots
                .build();

            let collider_handle = self.collider_set.insert_with_parent(
                collider,
                rigid_body_handle,
                &mut self.rigid_body_set,
            );

            self.particle_filter.points.push(collider_handle);
        }
    }

    /// Generate a walkable point near this point
    pub fn pf_random_point_near(
        &self,
        rng: &mut ThreadRng,
        grid: &ComputedGrid,
        point: Point2<u8>,
    ) -> Isometry2<f32> {
        let distance = rng.gen_range(0.0..self.particle_filter.spread).floor() as usize;
        let mut node = point;
        for _ in 0..distance {
            let neighbors = grid.neighbors(&node);
            node = neighbors[rng.gen_range(0..neighbors.len())];
        }

        self.pf_random_point_at(rng, grid, node)
    }

    /// Generate a completely random walkable point
    pub fn pf_random_point(&self, rng: &mut ThreadRng, grid: &ComputedGrid) -> Isometry2<f32> {
        // find a random walkable node
        let node = rng.gen_range(0..grid.walkable_nodes().len());
        let node = grid.walkable_nodes()[node];

        self.pf_random_point_at(rng, grid, node)
    }

    /// Generate a random valid point around a certain walkable square
    pub fn pf_random_point_at(
        &self,
        rng: &mut ThreadRng,
        grid: &ComputedGrid,
        node: Point2<u8>,
    ) -> Isometry2<f32> {
        // the central square (radius r) is where pacbot could be placed if there were walls all around
        let r = 1.0 - self.robot_specifications.collider_radius;

        // if r > 0.5, some of the cells are overlapping - cut off the edges of the central square
        if r >= 0.5 {
            let mut left_bottom = Point2::new(node.x as f32 - r, node.y as f32 - r);
            let mut right_top = Point2::new(node.x as f32 + r, node.y as f32 + r);

            if grid.next(&node, &Direction::Up).is_some() {
                right_top.y = node.y as f32 + 0.5;
            }
            if grid.next(&node, &Direction::Down).is_some() {
                left_bottom.y = node.y as f32 - 0.5;
            }
            if grid.next(&node, &Direction::Left).is_some() {
                left_bottom.x = node.x as f32 - 0.5;
            }
            if grid.next(&node, &Direction::Right).is_some() {
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
                if grid.next(&node, direction).is_some() {
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

    pub fn pf_update(&mut self, rng: &mut ThreadRng, grid: &ComputedGrid, position: Point2<u8>) {
        // TODO sort list by accuracy

        // randomize the last 'random' points
        for i in self.particle_filter.points.len() - self.particle_filter.random
            ..self.particle_filter.points.len()
        {
            let point = self.pf_random_point(rng, grid);
            self.rigid_body_set
                .get_mut(
                    self.collider_set
                        .get(self.particle_filter.points[i])
                        .unwrap()
                        .parent()
                        .unwrap(),
                )
                .unwrap()
                .set_position(point, true);
        }

        // randomize the last 'purge' points near the best guess
        for i in self.particle_filter.points.len()
            - self.particle_filter.random
            - self.particle_filter.purge
            ..self.particle_filter.points.len() - self.particle_filter.random
        {
            let point = self.pf_random_point_near(rng, grid, position);
            self.rigid_body_set
                .get_mut(
                    self.collider_set
                        .get(self.particle_filter.points[i])
                        .unwrap()
                        .parent()
                        .unwrap(),
                )
                .unwrap()
                .set_position(point, true);
        }
    }
}
