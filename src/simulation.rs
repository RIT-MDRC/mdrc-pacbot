use crate::grid::ComputedGrid;
use crate::robot::Robot;
use rapier2d::dynamics::{IntegrationParameters, RigidBodySet};
use rapier2d::geometry::{BroadPhase, NarrowPhase};
use rapier2d::na::{vector, Isometry2, Vector2};
use rapier2d::prelude::{
    CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, ImpulseJointSet, IslandManager,
    MultibodyJointSet, PhysicsPipeline, QueryPipeline, RigidBodyBuilder,
};

const GHOST_WIDTH: f32 = 0.45;

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

    robots: Vec<ColliderHandle>,
    ghosts: Vec<ColliderHandle>,
}

impl PacbotSimulation {
    fn new(grid: ComputedGrid) -> Self {
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        for wall in grid.walls() {
            let rigid_body = RigidBodyBuilder::fixed()
                .translation(vector![
                    (wall.right_top.x + wall.left_bottom.x) / 2,
                    (wall.right_top.y + wall.left_bottom.y) / 2,
                ])
                .build();

            let rigid_body_handle = rigid_body_set.insert(rigid_body);

            let collider = ColliderBuilder::cuboid(
                (wall.right_top.x - wall.left_bottom.x) / 2,
                (wall.right_top.y - wall.left_bottom.y) / 2,
            )
            .build();

            collider_set.insert_with_parent(collider, rigid_body_handle, &mut rigid_body_set);
        }

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

            robots: vec![],
            ghosts: vec![],
        }
    }

    fn step(&mut self) {
        self.physics_pipeline.step(
            &vector![0., 0.],
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

        self.query_pipeline
            .update(&self.rigid_body_set, &self.collider_set);
    }

    fn add_robot(&mut self, robot: &Robot, position: Isometry2<f32>) -> ColliderHandle {
        let rigid_body = RigidBodyBuilder::dynamic().position(position).build();
        let rigid_body_handle = self.rigid_body_set.insert(rigid_body);

        let collider = ColliderBuilder::ball(robot.collider_radius)
            .density(robot.density)
            .build();

        let collider_handle = self.collider_set.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut self.rigid_body_set,
        );

        self.robots.push(collider_handle);

        collider_handle
    }

    fn add_ghost(&mut self, position: Vector2<f32>) {
        let collider = ColliderBuilder::ball(GHOST_WIDTH).sensor(true);
    }
}
