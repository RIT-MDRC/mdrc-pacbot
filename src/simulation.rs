use crate::grid::ComputedGrid;
use crate::robot::Robot;
use anyhow::anyhow;
use anyhow::Result;
use rapier2d::dynamics::{IntegrationParameters, RigidBodySet};
use rapier2d::geometry::{BroadPhase, NarrowPhase};
use rapier2d::na::{ComplexField, Isometry2, Point2, Vector2};
use rapier2d::prelude::*;

const GROUP_WALL: u32 = 1;
const GROUP_ROBOT: u32 = 2;

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
}

impl PacbotSimulation {
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
        }
    }

    pub fn step(&mut self) {
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

    pub fn get_collider_position(&mut self, handle: ColliderHandle) -> Option<&Isometry2<f32>> {
        let rigid_body_handle = self.collider_set.get(handle)?.parent()?;
        Some(self.rigid_body_set.get(rigid_body_handle)?.position())
    }

    pub fn cast_ray(&mut self, ray: Ray, max_toi: Real) -> Option<Point<Real>> {
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
            // println!("{:?}", ray.point_at(toi));
            // println!("{:?}", self.primary_robot);
            // The first collider hit has the handle `handle` and it hit after
            // the ray travelled a distance equal to `ray.dir * toi`.
            return Some(ray.point_at(toi)); // Same as: `ray.origin + ray.dir * toi`
        }

        Some(ray.point_at(max_toi))
    }

    pub fn get_primary_robot_position(&mut self) -> &Isometry2<f32> {
        self.get_collider_position(self.primary_robot).unwrap()
    }

    pub fn get_primary_robot_rays(&mut self) -> Vec<Option<(Point<Real>, Point<Real>)>> {
        let sensors = self.robot_specifications.distance_sensors.clone();

        let iso = self
            .get_collider_position(self.primary_robot)
            .unwrap()
            .to_owned();

        // println!("{:?} {}", iso, iso.rotation.angle());

        sensors
            .iter()
            .map(|sensor| {
                // println!(
                //     "{:?}",
                //     iso.translation.transform_point(&sensor.relative_position)
                // );
                // println!("{:?}", (iso.rotation * sensor.relative_direction).angle());
                // println!(
                //     "{:?}",
                //     (iso.rotation * sensor.relative_direction)
                //         .transform_vector(&Vector2::new(1.0, 0.0))
                // );
                if let Some(p) = self.cast_ray(
                    Ray::new(
                        iso.translation.transform_point(&sensor.relative_position),
                        (iso.rotation * sensor.relative_direction)
                            .transform_vector(&Vector2::new(1.0, 0.0)),
                    ),
                    sensor.max_range,
                ) {
                    // println!("{:?}", p);
                    return Some((
                        iso.translation.transform_point(&sensor.relative_position),
                        p,
                    ));
                }
                None
            })
            .collect()
    }
}
