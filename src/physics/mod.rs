//! Handles all physics related operations

mod particle_filter;
mod raycast_grid;

use crate::grid::standard_grids::StandardGrid;
use crate::grid::{facing_direction, ComputedGrid, IntLocation};
use crate::network::PacbotSensors;
use crate::pathing::TargetVelocity;
use crate::physics::particle_filter::{ParticleFilter, ParticleFilterOptions};
use crate::robot::Robot;
use crate::util::stopwatch::Stopwatch;
use crate::{CvPositionSource, PacmanGameState, UserSettings};
use bevy::time::Time;
use bevy_ecs::prelude::*;
use pacbot_rs::location::LocationState;
use rapier2d::na::{Isometry2, Point2, Vector2};
use rapier2d::prelude::*;
use std::f32::consts::FRAC_PI_2;

use self::particle_filter::FilterPoint;

/// Rapier interaction group representing all walls
const GROUP_WALL: u32 = 1;
/// Rapier interaction group representing all robots
const GROUP_ROBOT: u32 = 2;

/// Small information generated by the physics engine
#[derive(Resource, Default)]
pub struct LightPhysicsInfo {
    /// The position used to simulate physics interactions - `None` in competition
    pub real_pos: Option<Isometry2<f32>>,
    /// The best guess position from the particle filter - `None` if particle filter is disabled
    pub pf_pos: Option<Isometry2<f32>>,
    /// Simulated distance sensor rays emanating from real_pos
    pub real_pos_rays: Vec<(Point2<f32>, Point2<f32>)>,
    /// Simulated distance sensor rays emanating from pf_pos
    pub pf_pos_rays: Vec<(Point2<f32>, Point2<f32>)>,
    /// Exposed particle filter points
    pub pf_points: Vec<FilterPoint>,
}

/// Tracks the performance of the physics engine
#[derive(Resource)]
pub struct PhysicsStopwatch(pub Stopwatch);

/// Tracks the performance of the particle filter
#[derive(Resource)]
pub struct ParticleFilterStopwatch(pub Stopwatch);

/// Handles all physics related operations
#[derive(Resource)]
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
        )
    }
}

/// Updates Pacbot's location from the simulation to the game engine
pub fn update_game_state_pacbot_loc(
    simulation: Res<PacbotSimulation>,
    grid: Res<ComputedGrid>,
    mut pacman_state: ResMut<PacmanGameState>,
    settings: Res<UserSettings>,
) {
    if settings.go_server_address.is_none() {
        let pacbot_location = simulation.get_primary_robot_position();
        let old_pacbot_location = IntLocation {
            row: pacman_state.0.get_state().pacman_loc.row,
            col: pacman_state.0.get_state().pacman_loc.col,
        };
        if let Some(pacbot_location) =
            grid.node_nearest(pacbot_location.translation.x, pacbot_location.translation.y)
        {
            if old_pacbot_location == pacbot_location {
                return;
            }
            pacman_state.0.set_pacman_location(LocationState {
                row: pacbot_location.row,
                col: pacbot_location.col,
                dir: facing_direction(&old_pacbot_location, &pacbot_location) as u8,
            });
        } else {
            eprintln!(
                "Could not convert location to grid space: {:?}",
                pacbot_location
            );
        }
    }
}

/// Steps the simulation
pub fn run_simulation(
    mut simulation: ResMut<PacbotSimulation>,
    mut phys_stopwatch: ResMut<PhysicsStopwatch>,
    target_velocity: Res<TargetVelocity>,
    time: Res<Time>,
    mut settings: ResMut<UserSettings>,
) {
    if let Some(pos) = settings.kidnap_position.take() {
        let rigid_body = simulation.get_robot_rigid_body();
        rigid_body.set_position(
            Isometry2::new(
                Vector2::new(pos.row as f32, pos.col as f32),
                rigid_body.rotation().angle(),
            ),
            true,
        );
    }
    phys_stopwatch.0.start();
    // translate target velocity by pf rotation, then un-translate it by real rotation
    // to simulate what would happen in the real world
    let x = target_velocity.0.x;
    let y = target_velocity.0.y;
    let mag = (x.powi(2) + y.powi(2)).sqrt();
    let pf_angle = simulation.pf_best_guess().loc.rotation.angle();
    let angle = y.atan2(x);
    let target_velocity_rot = angle - pf_angle + FRAC_PI_2;

    let mut target_vector = target_velocity.0;
    target_vector.x = mag * target_velocity_rot.cos();
    target_vector.y = mag * target_velocity_rot.sin();

    simulation.set_target_robot_velocity((target_vector, target_velocity.1));
    simulation.step(time.delta_seconds());
    phys_stopwatch.0.mark_segment("Step simulation");
}

/// Steps the particle filter
pub fn run_particle_filter(
    mut simulation: ResMut<PacbotSimulation>,
    mut pf_stopwatch: ResMut<ParticleFilterStopwatch>,
    grid: Res<ComputedGrid>,
    sensors: Res<PacbotSensors>,
    settings: Res<UserSettings>,
    time: Res<Time>,
    game_engine: Res<PacmanGameState>,
) {
    if settings.enable_pf {
        simulation
            .particle_filter
            .set_options(ParticleFilterOptions {
                points: settings.pf_total_points,
            });

        simulation.robot_specifications = settings.robot.clone();
        simulation.particle_filter.set_robot(settings.robot.clone());

        // Update particle filter
        let rigid_body = simulation.get_robot_rigid_body();
        let vel_lin = *rigid_body.linvel();
        let vel_ang = rigid_body.angvel();
        let angle = rigid_body.rotation().angle();
        // Rotate vel_lin to align with robot rotation
        let local_vel = Vector2::new(
            vel_lin.x * (-angle).cos() - vel_lin.y * (-angle).sin(),
            vel_lin.x * (-angle).sin() + vel_lin.y * (-angle).cos(),
        );
        let cv_position = match settings.cv_position {
            CvPositionSource::GameState => game_engine.0.get_state().pacman_loc,
            CvPositionSource::ParticleFilter => {
                let pf_pos = simulation.pf_best_guess();
                let pos = grid.node_nearest(pf_pos.loc.translation.x, pf_pos.loc.translation.y);
                match pos {
                    None => game_engine.0.get_state().pacman_loc,
                    Some(x) => LocationState {
                        row: x.row,
                        col: x.col,
                        dir: 0,
                    },
                }
            }
            CvPositionSource::Constant(row, col) => LocationState::new(row, col, 0),
        };
        simulation.pf_update(
            (local_vel, vel_ang),
            time.delta_seconds(),
            &mut pf_stopwatch.0,
            &sensors,
            &settings,
            cv_position,
        );
    }
}

/// Transfers information from the simulation to the shared phys_info resource
pub fn update_physics_info(
    mut simulation: ResMut<PacbotSimulation>,
    mut sensors: ResMut<PacbotSensors>,
    mut phys_info: ResMut<LightPhysicsInfo>,
    settings: Res<UserSettings>,
) {
    let primary_position = *simulation.get_primary_robot_position();
    let pf_position = simulation.pf_best_guess();
    let rays = simulation.get_distance_sensor_rays(primary_position);
    if !settings.sensors_from_robot {
        #[allow(clippy::needless_range_loop)]
        for i in 0..sensors.distance_sensors.len() {
            let (a, b) = rays[i];
            sensors.distance_sensors[i] = (((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
                * 88.9)
                .round()
                .min(255.0) as u8;
        }
    }
    let best_guess = if settings.enable_pf {
        simulation.pf_best_guess().loc
    } else {
        *simulation.get_primary_robot_position()
    };
    let pf_pos_rays = if settings.enable_pf {
        simulation.get_distance_sensor_rays(pf_position.loc)
    } else {
        rays.clone()
    };
    *phys_info = LightPhysicsInfo {
        real_pos: Some(*simulation.get_primary_robot_position()),
        pf_pos: Some(best_guess),
        real_pos_rays: rays,
        pf_pos_rays,
        pf_points: if settings.enable_pf {
            simulation.particle_filter.points(settings.pf_gui_points)
        } else {
            vec![]
        },
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
    pub fn new(grid: ComputedGrid, robot: Robot, robot_position: Isometry2<f32>) -> Self {
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
            ParticleFilterOptions { points: 10 },
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
    /// let dt = 1.0 / 60.0;
    ///
    /// // in an infinite loop
    /// simulation.step(dt);
    /// ```
    pub fn step(&mut self, dt: f32) {
        self.integration_parameters.set_inv_dt(1.0 / dt);
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

    /// Get the primary robot's rigid body
    pub fn get_robot_rigid_body(&mut self) -> &mut RigidBody {
        self.rigid_body_set
            .get_mut(
                self.collider_set
                    .get(self.primary_robot)
                    .unwrap()
                    .parent()
                    .unwrap(),
            )
            .unwrap()
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

        let toi = self.particle_filter.raycast_grid().raycast(ray, max_toi);
        ray.point_at(toi)
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
    pub fn get_distance_sensor_rays(
        &mut self,
        pacbot: Isometry2<f32>,
    ) -> Vec<(Point<Real>, Point<Real>)> {
        let sensors = self.robot_specifications.distance_sensors.clone();

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
    pub fn pf_best_guess(&self) -> FilterPoint {
        self.particle_filter.best_guess()
    }

    /// Get the best 'count' particle filter points
    pub fn pf_points(&self, count: usize) -> Vec<FilterPoint> {
        self.particle_filter.points(count)
    }
}
