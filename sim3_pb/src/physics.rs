use crate::Wall;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use core_pb::constants::{INCHES_PER_GU, MM_PER_GU};
use core_pb::grid::standard_grid::StandardGrid;

struct RobotShapeWheel {
    center: Vec3,
    radius: f32,
    thickness: f32,
}

struct RobotShape {
    collider_size: Vec3,
    collider_z: f32,
    wheels: Vec<RobotShapeWheel>,
}

pub fn spawn_walls(commands: &mut Commands, grid: StandardGrid) {
    let grid = grid.compute_grid();

    let inches = |x: f32| x / INCHES_PER_GU;

    // Create the floor
    commands
        .spawn(Collider::cuboid(16.0, 16.0, 0.10))
        .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
        .insert(TransformBundle::from(Transform::from_xyz(
            16.0, 16.0, -0.10,
        )))
        .insert(Wall);
    //
    // commands
    //     .spawn(Collider::cuboid(32.0, 32.0, 0.01))
    //     .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
    //     .insert(TransformBundle::from(Transform::from_xyz(16.0, 16.0, 0.0)))
    //     .insert(Wall);
    // commands
    //     .spawn(Collider::cuboid(1.0, 1.0, 0.01))
    //     .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
    //     .insert(TransformBundle::from(Transform::from_xyz(1.0, 1.0, 0.0)))
    //     .insert(Wall);
    //
    // Create the walls
    for wall in grid.walls() {
        commands
            .spawn(Collider::cuboid(
                (wall.bottom_right.x as f32 * 1.0 - wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 - wall.top_left.y as f32 * 1.0) / 2.0,
                inches(2.0),
            ))
            .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
            .insert(TransformBundle::from(Transform::from_xyz(
                (wall.bottom_right.x as f32 * 1.0 + wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 + wall.top_left.y as f32 * 1.0) / 2.0,
                inches(2.0),
            )))
            .insert(Wall);
    }

    // Create the robot
    let wheel_radius = 16.0 / MM_PER_GU;
    let wheel_thickness = 6.5 / MM_PER_GU;
    let robot = RobotShape {
        collider_size: Vec3::new(inches(3.5), inches(3.0), wheel_radius),
        collider_z: wheel_radius,
        wheels: vec![
            RobotShapeWheel {
                center: Vec3::new(
                    inches(0.8),
                    inches(1.5) + wheel_thickness / 2.0 + inches(0.05),
                    wheel_radius,
                ),
                radius: wheel_radius,
                thickness: wheel_thickness,
            },
            RobotShapeWheel {
                center: Vec3::new(
                    inches(0.8),
                    -(inches(1.5) + wheel_thickness / 2.0 + inches(0.05)),
                    wheel_radius,
                ),
                radius: wheel_radius,
                thickness: wheel_thickness,
            },
        ],
    };
    let robot_pos = Vec3::new(1.0, 1.0, 1.0);

    // Draw collider rectangle
    let new_robot = commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::cuboid(
            robot.collider_size.x / 2.0,
            robot.collider_size.y / 2.0,
            robot.collider_size.z / 2.0,
        ))
        .insert(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1))
        .insert(TransformBundle::from(Transform::from_xyz(
            robot_pos.x,
            robot_pos.y,
            robot_pos.z + robot.collider_z,
        )))
        .insert(Velocity::default())
        .id();

    for wheel in &robot.wheels {
        let revolute_joint = RevoluteJointBuilder::new(Vec3::Y)
            .local_anchor1(wheel.center + Vec3::new(0.0, 0.0, -robot.collider_z));

        commands
            .spawn(RigidBody::Dynamic)
            .insert(Collider::cylinder(wheel_thickness / 2.0, wheel_radius))
            .insert(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1))
            .insert(ImpulseJoint::new(new_robot, revolute_joint));
    }
}
