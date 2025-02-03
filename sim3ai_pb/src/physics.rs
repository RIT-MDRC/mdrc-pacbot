use glam::Vec3;
use pid::Pid;
use rapier3d::na::{Isometry3, Vector3};
use rapier3d::prelude::*;
use sim3_pb::physics::RobotShape;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const GRAVITY: Vector3<f32> = Vector3::new(0.0, 0.0, -9.81);
static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct GymRobotHandle(usize);

#[derive(Copy, Clone, Debug)]
pub struct GymRobotWheel {
    rigid_body_handle: RigidBodyHandle,
    impulse_joint_handle: ImpulseJointHandle,
    pid: Pid<f32>,
}

#[derive(Clone, Debug)]
pub struct GymRobot {
    handle: GymRobotHandle,
    rigid_body_handle: RigidBodyHandle,
    wheels: Vec<GymRobotWheel>,
}

pub struct Pacbot3dGym {
    steps: usize,
    robots: HashMap<GymRobotHandle, GymRobot>,

    // rapier simulation structs
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
}

impl Pacbot3dGym {
    pub fn new() -> Self {
        Self {
            steps: 0,
            robots: HashMap::new(),

            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn step_physics(&mut self) {
        self.steps += 1;

        self.physics_pipeline.step(
            &GRAVITY,
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
    }

    fn make_pid() -> Pid<f32> {
        let mut pid = Pid::new(0.0, 0.01);
        pid.p(0.01, 0.01);
        pid
    }

    pub fn delete_robot(&mut self, robot_handle: GymRobotHandle) {
        if let Some(mut robot) = self.robots.remove(&robot_handle) {
            self.rigid_body_set.remove(
                robot.rigid_body_handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
            // todo maybe remove wheels?
        }
    }

    pub fn spawn_robot(&mut self, at: Isometry3<f32>, shape: &RobotShape) -> GymRobotHandle {
        let pos_height = Vec3::ZERO.with_z(shape.collider_z);

        // Main rigid body
        let rigid_body = RigidBodyBuilder::dynamic()
            // .position(at + Isometry3::new(pos_height., ))
            .build();
        let robot_rigid_body_handle = self.rigid_body_set.insert(rigid_body);

        // Main collider rectangle
        let collider = ColliderBuilder::cuboid(
            shape.collider_size.x / 2.0,
            shape.collider_size.y / 2.0,
            shape.collider_size.z / 2.0,
        )
        .collision_groups(InteractionGroups::new(Group::GROUP_2, Group::GROUP_1))
        .build();
        let _collider_handle = self.collider_set.insert_with_parent(
            collider,
            robot_rigid_body_handle,
            &mut self.rigid_body_set,
        );

        // Wheels
        let wheels = shape
            .wheels
            .iter()
            .map(|wheel| {
                let rigid_body = RigidBodyBuilder::dynamic()
                    .position(wheel.center + at)
                    .build();
                let rigid_body_handle = self.rigid_body_set.insert(rigid_body);

                let collider = ColliderBuilder::cylinder(wheel.thickness / 2.0, wheel.radius)
                    .collision_groups(InteractionGroups::new(Group::GROUP_2, Group::GROUP_1))
                    .build();
                let _collider_handle = self.collider_set.insert_with_parent(
                    collider,
                    rigid_body_handle,
                    &mut self.rigid_body_set,
                );

                let joint = RevoluteJointBuilder::new(UnitVector::new_normalize(Vector3::new(
                    0.0, 1.0, 0.0,
                )))
                .local_anchor1(Point::new(
                    wheel.center.x + -pos_height.x,
                    wheel.center.y + -pos_height.y,
                    wheel.center.z + -pos_height.z,
                ));
                let impulse_joint_handle = self.impulse_joint_set.insert(
                    robot_rigid_body_handle,
                    rigid_body_handle,
                    joint,
                    true,
                );

                let mut pid = Pid::new(0.0, 0.01);
                pid.p(0.01, 0.01);

                GymRobotWheel {
                    rigid_body_handle,
                    impulse_joint_handle,
                    pid,
                }
            })
            .collect::<Vec<_>>();

        // todo casters/distance sensors/distance sensor markers

        let handle = GymRobotHandle(NEXT_HANDLE.fetch_add(1, Ordering::Relaxed));

        let gym_robot = GymRobot {
            handle,
            rigid_body_handle: robot_rigid_body_handle,
            wheels,
        };
        self.robots.insert(handle, gym_robot);

        handle
    }
}
