//! Handles all physics related operations

mod particle_filter;

use crate::constants::{
    NUM_PARTICLE_FILTER_POINTS, PARTICLE_FILTER_ELITE, PARTICLE_FILTER_PURGE,
    PARTICLE_FILTER_RANDOM,
};
use crate::grid::standard_grids::StandardGrid;
use crate::grid::ComputedGrid;
use crate::physics::particle_filter::{ParticleFilter, ParticleFilterOptions};
use crate::robot::Robot;
use rapier2d::dynamics::{IntegrationParameters, RigidBodySet};
use rapier2d::geometry::{BroadPhase, NarrowPhase};
use rapier2d::na::{Isometry2, Vector2};
use rapier2d::prelude::*;
use std::sync::{Arc, Mutex};

/// Rapier interaction group representing all walls
const GROUP_WALL: u32 = 1;
/// Rapier interaction group representing all robots
const GROUP_ROBOT: u32 = 2;

/// Handles all physics related operations
pub struct PacbotSimulation {
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,

    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,

    query_pipeline: QueryPipeline,
    query_pipeline_updated: bool,

    robot_specifications: Robot,
    primary_robot: ColliderHandle,
    robot_target_velocity: (Vector2<f32>, f32),

    particle_filter: ParticleFilter,
}

impl Default for PacbotSimulation {
    /// Creates a simulation with GRID_PACMAN, the default Robot, and starting position (14, 7)
    fn default() -> Self {
        Self::new(
            StandardGrid::Pacman.compute_grid(),
            Robot::default(),
            StandardGrid::Pacman.get_default_pacbot_isometry(),
            Arc::new(Mutex::new(vec![
                Some(0.0);
                Robot::default().distance_sensors.len()
            ])),
        )
    }
}

impl PacbotSimulation {
    /// Create a new simulation on a ComputedGrid with a starting Robot and position
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use pacbot_rs::variables::PACMAN_SPAWN_LOC;
    /// use rand::rngs::ThreadRng;
    /// use rapier2d::na::{Isometry2, Vector2};
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::robot::Robot;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Pacman.compute_grid();
    /// let robot = Robot::default();
    /// let distance_sensors = Arc::new(Mutex::new(vec![Some(0.0); Robot::default().distance_sensors.len()]));
    /// let starting_position = Isometry2::new(Vector2::new(PACMAN_SPAWN_LOC.row as f32, PACMAN_SPAWN_LOC.col as f32), 0.0);
    /// let mut simulation = PacbotSimulation::new(grid, robot, starting_position, distance_sensors);
    /// ```
    pub fn new(
        grid: ComputedGrid,
        robot: Robot,
        robot_position: Isometry2<f32>,
        distance_sensors: Arc<Mutex<Vec<Option<f32>>>>,
    ) -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // add walls
        for wall in grid.walls() {
            let rigid_body = RigidBodyBuilder::fixed()
                .translation(Vector2::new(
                    (wall.top_left.row + wall.bottom_right.row) as f32 / 2.0,
                    (wall.top_left.col + wall.bottom_right.col) as f32 / 2.0,
                ))
                .build();

            let rigid_body_handle = rigid_body_set.insert(rigid_body);

            let collider = ColliderBuilder::cuboid(
                (wall.bottom_right.row - wall.top_left.row) as f32 / 2.0,
                (wall.bottom_right.col - wall.top_left.col) as f32 / 2.0,
            )
            .collision_groups(InteractionGroups::new(GROUP_WALL.into(), u32::MAX.into()))
            .build();

            collider_set.insert_with_parent(collider, rigid_body_handle, &mut rigid_body_set);
        }

        // add robot
        let rigid_body = RigidBodyBuilder::dynamic().position(robot_position).build();
        let rigid_body_handle = rigid_body_set.insert(rigid_body);

        let collider = ColliderBuilder::ball(robot.collider_radius)
            .density(robot.density)
            .collision_groups(InteractionGroups::new(
                GROUP_ROBOT.into(),
                GROUP_WALL.into(),
            )) // allows robot to only interact with walls, not other robots
            .build();

        let collider_handle =
            collider_set.insert_with_parent(collider, rigid_body_handle, &mut rigid_body_set);

        let query_pipeline = QueryPipeline::new();

        let particle_filter = ParticleFilter::new(
            grid,
            robot.to_owned(),
            robot_position,
            distance_sensors,
            ParticleFilterOptions {
                points: NUM_PARTICLE_FILTER_POINTS,
                elite: PARTICLE_FILTER_ELITE,
                purge: PARTICLE_FILTER_PURGE,
                random: PARTICLE_FILTER_RANDOM,
                spread: 2.5,
                elitism_bias: 1.0,
                genetic_translation_limit: 0.1,
                genetic_rotation_limit: 0.1,
            },
        );

        Self {
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),

            rigid_body_set,
            collider_set,

            query_pipeline,
            query_pipeline_updated: false,

            robot_specifications: robot,
            primary_robot: collider_handle,
            robot_target_velocity: (Vector2::new(0.0, 0.0), 0.0),

            particle_filter,
        }
    }

    /// Update the physics simulation
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// // in an infinite loop
    /// simulation.step();
    /// ```
    pub fn step(&mut self) {
        self.step_target_velocity();

        self.physics_pipeline.step(
            &Vector2::new(0., 0.),
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );

        self.query_pipeline_updated = false;
    }

    /// Apply an impulse to the primary robot based on robot_target_velocity
    fn step_target_velocity(&mut self) {
        let rigid_body = self
            .rigid_body_set
            .get_mut(
                self.collider_set
                    .get(self.primary_robot)
                    .unwrap()
                    .parent()
                    .unwrap(),
            )
            .unwrap();

        rigid_body.apply_impulse(self.robot_target_velocity.0 - rigid_body.linvel(), true);
        rigid_body.apply_torque_impulse(
            0.1 * (self.robot_target_velocity.1 - rigid_body.angvel()),
            true,
        );
    }

    /// Get the [`Isometry`] for a given [`ColliderHandle`]
    ///
    /// May return [`None`] under any of the following conditions:
    /// - there is no matching collider
    /// - the collider in question has no associated rigid body
    /// - the rigid body associated with the collider is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::geometry::ColliderHandle;
    /// use rapier2d::na::{Isometry2, Point2, Vector2};
    /// use rapier2d::prelude::Rotation;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// // in an infinite loop
    /// let collider: ColliderHandle = simulation.get_primary_robot_collider();
    /// let isometry: &Isometry2<f32> = simulation.get_collider_position(collider).unwrap();
    /// let position: Point2<f32> = isometry.translation.transform_point(&Point2::new(0.0, 0.0));
    /// let rotation: Rotation<f32> = isometry.rotation;
    /// ```
    pub fn get_collider_position(&self, handle: ColliderHandle) -> Option<&Isometry2<f32>> {
        let rigid_body_handle = self.collider_set.get(handle)?.parent()?;
        Some(self.rigid_body_set.get(rigid_body_handle)?.position())
    }

    /// Cast a ray in the simulation
    ///
    /// Rays will pass through robots and only hit walls. It is recommended to normalize the
    /// Ray's direction so that max_toi represents a maximum distance. If the ray does not strike
    /// a wall within max_toi, the point at max_toi along the ray will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::{Point2, Vector2};
    /// use rapier2d::prelude::{Ray, Rotation};
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let pacbot_position = simulation.get_primary_robot_position();
    /// let positive_row = Vector2::new(1.0, 0.0);
    /// let ray = Ray::new(pacbot_position.translation.transform_point(&Point2::new(0.0, 0.0)), positive_row);
    ///
    /// assert_eq!(simulation.cast_ray(ray, 5.0), Point2::new(24.0, 13.0));
    /// assert_eq!(simulation.cast_ray(ray, 0.5), Point2::new(23.5, 13.0));
    ///
    /// let pos = Point2::new(29.0, 21.0);
    /// let positive_row = Vector2::new(1.0, 0.0);
    /// let ray = Ray::new(pos, positive_row);
    ///
    /// assert_eq!(simulation.cast_ray(ray, 5.0), Point2::new(30.0, 21.0));
    /// ```
    pub fn cast_ray(&mut self, ray: Ray, max_toi: Real) -> Point<Real> {
        if !self.query_pipeline_updated {
            self.query_pipeline
                .update(&self.rigid_body_set, &self.collider_set);
        }

        let filter = QueryFilter::new().groups(InteractionGroups::new(
            GROUP_ROBOT.into(),
            GROUP_WALL.into(),
        ));

        if let Some((_, toi)) = self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_toi,
            true,
            filter,
        ) {
            // The first collider hit has the handle `handle` and it hit after
            // the ray travelled a distance equal to `ray.dir * toi`.
            return ray.point_at(toi); // Same as: `ray.origin + ray.dir * toi`
        }

        ray.point_at(max_toi)
    }

    /// Get the collider for the primary robot
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::prelude::ColliderHandle;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let primary_collider: ColliderHandle = simulation.get_primary_robot_collider();
    /// ```
    pub fn get_primary_robot_collider(&self) -> ColliderHandle {
        self.primary_robot
    }

    /// Get the current position of the primary robot
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::math::Rotation;
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let isometry = simulation.get_primary_robot_position();
    /// let position: Point2<f32> = isometry.translation.transform_point(&Point2::new(0.0, 0.0));
    /// let rotation: Rotation<f32> = isometry.rotation;
    /// ```
    pub fn get_primary_robot_position(&self) -> &Isometry2<f32> {
        self.get_collider_position(self.primary_robot).unwrap()
    }

    /// Set the target velocity (translational and rotational) for the primary robot
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Vector2;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let w_key_pressed = true;
    /// if w_key_pressed {
    ///     simulation.set_target_robot_velocity((Vector2::new(0.0, 1.0), 0.0));
    /// }
    /// ```
    pub fn set_target_robot_velocity(&mut self, v: (Vector2<f32>, f32)) {
        self.robot_target_velocity = v;
    }

    /// Get the rays coming out of the primary robot, representing the theoretical readings from
    /// its distance sensors.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::f32::consts::PI;
    /// use mdrc_pacbot_util::robot::Robot;
    /// use mdrc_pacbot_util::physics::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let robot = Robot::default();
    /// let rays = simulation.get_primary_robot_rays();
    ///
    /// for i in 0..robot.distance_sensors.len() {
    ///     let sensor = robot.distance_sensors[i];
    ///
    ///     let sensor_position = rays[i].0;
    ///     let hit_point = rays[i].1;
    ///
    ///     let difference = hit_point - sensor_position;
    ///
    ///     let rotated_direction = PI / 2.0 + sensor.relative_direction;
    ///
    ///     assert_eq!((rotated_direction.cos() >= 0.1), (difference.x >= 0.1));
    ///     assert_eq!((rotated_direction.sin() >= 0.1), (difference.y >= 0.1));
    /// }
    /// ```
    pub fn get_primary_robot_rays(&mut self) -> Vec<(Point<Real>, Point<Real>)> {
        let sensors = self.robot_specifications.distance_sensors.clone();

        let pacbot = self
            .get_collider_position(self.primary_robot)
            .unwrap()
            .to_owned();

        sensors
            .iter()
            .map(|sensor| {
                let p = self.cast_ray(
                    Ray::new(
                        pacbot.translation.transform_point(
                            &pacbot.rotation.transform_point(&sensor.relative_position),
                        ),
                        (pacbot.rotation * Rotation::new(sensor.relative_direction))
                            .transform_vector(&Vector2::new(1.0, 0.0)),
                    ),
                    sensor.max_range,
                );
                (
                    pacbot.translation.transform_point(
                        &pacbot.rotation.transform_point(&sensor.relative_position),
                    ),
                    p,
                )
            })
            .collect()
    }

    /// Get the particle filter's best guess position
    pub fn pf_best_guess(&self) -> Isometry2<f32> {
        self.particle_filter.best_guess()
    }

    /// Get the best 'count' particle filter points
    pub fn pf_points(&self, count: usize) -> Vec<Isometry2<f32>> {
        self.particle_filter.points(count)
    }
}
