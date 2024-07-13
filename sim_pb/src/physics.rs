use crate::driving::SimRobot;
use crate::{MyApp, Robot, Wall, ROBOT_RADIUS};
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy_rapier2d::geometry::{Collider, CollisionGroups, Group};
use bevy_rapier2d::na::Vector2;
use bevy_rapier2d::prelude::*;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::RobotName;

pub fn spawn_walls(commands: &mut Commands, grid: StandardGrid) {
    let grid = grid.compute_grid();

    // Create the walls
    for wall in grid.walls() {
        commands
            .spawn(Collider::cuboid(
                (wall.bottom_right.x as f32 * 1.0 - wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 - wall.top_left.y as f32 * 1.0) / 2.0,
            ))
            .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
            .insert(TransformBundle::from(Transform::from_xyz(
                (wall.bottom_right.x as f32 * 1.0 + wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 + wall.top_left.y as f32 * 1.0) / 2.0,
                0.0,
            )))
            .insert(Wall);
    }
}

impl MyApp {
    pub fn spawn_robot(&mut self, commands: &mut Commands, name: RobotName) {
        let pos = self.standard_grid.get_default_pacbot_isometry().translation;

        let new_robot = commands
            .spawn(RigidBody::Dynamic)
            .insert(Collider::ball(ROBOT_RADIUS))
            .insert(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1))
            .insert(TransformBundle::from(Transform::from_xyz(
                pos.x, pos.y, 0.0,
            )))
            .insert(ExternalImpulse::default())
            .insert(Velocity::default())
            .insert(Robot {
                name,
                wasd_target_vel: None,
            })
            .id();

        let sim_robot = SimRobot::start(name);

        self.robots[name as usize] = Some((new_robot, sim_robot));
        self.selected_robot = name;
    }

    pub fn despawn_robot(&mut self, name: RobotName, commands: &mut Commands) {
        if let Some((entity, sim_robot)) = self.robots[name as usize].take() {
            commands.entity(entity).despawn();
            sim_robot.write().unwrap().destroy();
        }
    }

    pub fn apply_robots_target_vel(
        &mut self,
        robots: &mut Query<(
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut ExternalImpulse,
            &mut Robot,
        )>,
    ) {
        for (_, _, v, mut imp, robot) in robots {
            let mut target_vel = robot
                .wasd_target_vel
                .unwrap_or((Vector2::new(0.0, 0.0), 0.0));
            let move_scale = 4.0;
            if target_vel.0 != Vector2::new(0.0, 0.0) {
                target_vel.0 = target_vel.0.normalize() * move_scale;
            }
            imp.impulse.x = target_vel.0.x - v.linvel.x * 0.6;
            imp.impulse.y = target_vel.0.y - v.linvel.y * 0.6;
            imp.torque_impulse = target_vel.1 - v.angvel * 0.1;
        }
    }

    pub fn reset_grid(
        &mut self,
        walls: Query<(Entity, &Wall)>,
        robots: &mut Query<(
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut ExternalImpulse,
            &mut Robot,
        )>,
        commands: &mut Commands,
    ) {
        for wall in &walls {
            commands.entity(wall.0).despawn()
        }
        spawn_walls(commands, self.standard_grid);
        for (_, mut t, mut v, _, _) in robots {
            let pos = self.standard_grid.get_default_pacbot_isometry().translation;
            t.translation = Vec3::new(pos.x, pos.y, 0.0);
            v.linvel = Vect::ZERO;
            v.angvel = 0.0;
        }
    }
}
