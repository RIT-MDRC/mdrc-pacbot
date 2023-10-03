//! Handles all physics related operations

use crate::grid::ComputedGrid;
use crate::robot::Robot;
use crate::standard_grids::GRID_PACMAN;
use rapier2d::dynamics::{IntegrationParameters, RigidBodySet};
use rapier2d::geometry::{BroadPhase, NarrowPhase};
use rapier2d::na::{Isometry2, Vector2};
use rapier2d::prelude::*;

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
}

impl Default for PacbotSimulation {
    /// Creates a simulation with GRID_PACMAN, the default Robot, and starting position (14, 7)
    fn default() -> Self {
        let grid = ComputedGrid::try_from(GRID_PACMAN).unwrap();
        Self::new(
            grid,
            Robot::default(),
            Isometry2::new(Vector2::new(14.0, 7.0), 0.0),
        )
    }
}

impl PacbotSimulation {
    /// Create a new simulation on a ComputedGrid with a starting Robot and position
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::{Isometry2, Vector2};
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::robot::Robot;
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
    /// use mdrc_pacbot_util::standard_grids::GRID_PACMAN;
    ///
    /// let grid = ComputedGrid::try_from(GRID_PACMAN).unwrap();
    /// let robot = Robot::default();
    /// let starting_position = Isometry2::new(Vector2::new(14.0, 7.0), 0.0);
    /// let mut simulation = PacbotSimulation::new(grid, robot, starting_position);
    /// ```
    pub fn new(grid: ComputedGrid, robot: Robot, robot_position: Isometry2<f32>) -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // add walls
        for wall in grid.walls() {
            let rigid_body = RigidBodyBuilder::fixed()
                .translation(Vector2::new(
                    (wall.right_top.x + wall.left_bottom.x) / 2.0,
                    (wall.right_top.y + wall.left_bottom.y) / 2.0,
                ))
                .build();

            let rigid_body_handle = rigid_body_set.insert(rigid_body);

            let collider = ColliderBuilder::cuboid(
                (wall.right_top.x - wall.left_bottom.x) / 2.0,
                (wall.right_top.y - wall.left_bottom.y) / 2.0,
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

            query_pipeline: QueryPipeline::new(),
            query_pipeline_updated: false,

            robot_specifications: robot,
            primary_robot: collider_handle,
            robot_target_velocity: (Vector2::new(0.0, 0.0), 0.0),
        }
    }

    /// Update the physics simulation
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
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
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// // in an infinite loop
    /// let collider: ColliderHandle = simulation.get_primary_robot_collider();
    /// let isometry: &Isometry2<f32> = simulation.get_collider_position(collider).unwrap();
    /// let position: Point2<f32> = isometry.translation.transform_point(&Point2::new(0.0, 0.0));
    /// let rotation: Rotation<f32> = isometry.rotation;
    /// ```
    pub fn get_collider_position(&mut self, handle: ColliderHandle) -> Option<&Isometry2<f32>> {
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
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let pacbot_position = simulation.get_primary_robot_position();
    /// let positive_y = Vector2::new(0.0, 1.0);
    /// let ray = Ray::new(pacbot_position.translation.transform_point(&Point2::new(0.0, 0.0)), positive_y);
    ///
    /// assert_eq!(simulation.cast_ray(ray, 5.0), Point2::new(14.0, 8.0));
    /// assert_eq!(simulation.cast_ray(ray, 0.5), Point2::new(14.0, 7.5));
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
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
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
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
    /// let mut simulation = PacbotSimulation::default();
    ///
    /// let isometry = simulation.get_primary_robot_position();
    /// let position: Point2<f32> = isometry.translation.transform_point(&Point2::new(0.0, 0.0));
    /// let rotation: Rotation<f32> = isometry.rotation;
    /// ```
    pub fn get_primary_robot_position(&mut self) -> &Isometry2<f32> {
        self.get_collider_position(self.primary_robot).unwrap()
    }

    /// Set the target velocity (translational and rotational) for the primary robot
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Vector2;
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
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
    /// use mdrc_pacbot_util::robot::Robot;
    /// use mdrc_pacbot_util::simulation::PacbotSimulation;
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
    ///     assert_eq!((sensor.relative_direction.cos() >= 0.1), (difference.x >= 0.1));
    ///     assert_eq!((sensor.relative_direction.sin() >= 0.1), (difference.y >= 0.1));
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
                return (
                    pacbot.translation.transform_point(
                        &pacbot.rotation.transform_point(&sensor.relative_position),
                    ),
                    p,
                );
            })
            .collect()
    }
}
