use crate::{Wall, Wheel};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use core_pb::constants::{INCHES_PER_GU, MM_PER_GU};
use core_pb::grid::standard_grid::StandardGrid;
use pid::Pid;

struct RobotShapeWheel {
    center: Vec3,
    radius: f32,
    thickness: f32,
    motor: bool,
}

struct RobotCasterWheel {
    center: Vec3,
    radius: f32,
}

struct RobotDistanceSensor {
    center: Vec3,
    facing: Vec3,
}

struct RobotShape {
    collider_size: Vec3,
    collider_z: f32,
    wheels: Vec<RobotShapeWheel>,
    casters: Vec<RobotCasterWheel>,
    distance_sensors: Vec<RobotDistanceSensor>,
}

pub fn spawn_walls(commands: &mut Commands, grid: StandardGrid) {
    let grid = grid.compute_grid();

    let inches = |x: f32| x / INCHES_PER_GU;

    // Create the floor
    commands
        .spawn(Collider::cuboid(16.0, 16.0, 0.10))
        .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
        .insert(Transform::from_xyz(16.0, 16.0, -0.10))
        .insert(Wall);

    // Create the walls
    for wall in grid.walls() {
        commands
            .spawn(Collider::cuboid(
                (wall.bottom_right.x as f32 * 1.0 - wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 - wall.top_left.y as f32 * 1.0) / 2.0,
                inches(2.0),
            ))
            .insert(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2))
            .insert(Transform::from_xyz(
                (wall.bottom_right.x as f32 * 1.0 + wall.top_left.x as f32 * 1.0) / 2.0,
                (wall.bottom_right.y as f32 * 1.0 + wall.top_left.y as f32 * 1.0) / 2.0,
                inches(2.0),
            ))
            .insert(Wall);
    }

    // Create the robot
    let wheel_radius = 16.0 / MM_PER_GU;
    let wheel_thickness = 6.5 / MM_PER_GU;
    let robot = RobotShape {
        collider_size: Vec3::new(inches(4.0), inches(3.4), wheel_radius),
        collider_z: wheel_radius,
        wheels: vec![
            RobotShapeWheel {
                center: Vec3::new(
                    inches(0.77),
                    inches(1.7) + wheel_thickness / 2.0 + inches(0.05),
                    wheel_radius,
                ),
                radius: wheel_radius,
                thickness: wheel_thickness,
                motor: true,
            },
            RobotShapeWheel {
                center: Vec3::new(
                    inches(0.77),
                    -(inches(1.7) + wheel_thickness / 2.0 + inches(0.05)),
                    wheel_radius,
                ),
                radius: wheel_radius,
                thickness: wheel_thickness,
                motor: true,
            },
        ],
        casters: vec![RobotCasterWheel {
            center: Vec3::new(inches(-1.582), 0.0, 8.0 / MM_PER_GU),
            radius: 8.0 / MM_PER_GU,
        }],
        distance_sensors: vec![
            RobotDistanceSensor {
                center: Vec3::new(inches(2.0 - 0.482), 0.0, inches(1.5)),
                facing: Vec3::X,
            },
            RobotDistanceSensor {
                center: Vec3::new(
                    inches(2.0 - 0.482 - 0.759 + 0.238),
                    inches(0.853),
                    inches(1.5),
                ),
                facing: Vec3::Y,
            },
            RobotDistanceSensor {
                center: Vec3::new(
                    inches(2.0 - 0.482 - 0.759 + 0.238),
                    inches(-0.853),
                    inches(1.5),
                ),
                facing: Vec3::NEG_Y,
            },
            RobotDistanceSensor {
                center: Vec3::new(inches(-2.0 + 0.418), inches(0.853), inches(1.5)),
                facing: Vec3::Y,
            },
            RobotDistanceSensor {
                center: Vec3::new(inches(-2.0 + 0.418), inches(-0.853), inches(1.5)),
                facing: Vec3::NEG_Y,
            },
        ],
    };
    let robot_pos = Vec3::new(1.0, 1.0, 0.5);

    // Draw collider rectangle
    let new_robot = commands
        .spawn((
            RigidBody::Dynamic,
            Collider::cuboid(
                robot.collider_size.x / 2.0,
                robot.collider_size.y / 2.0,
                robot.collider_size.z / 2.0,
            ),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
            Transform::from_xyz(robot_pos.x, robot_pos.y, robot_pos.z + robot.collider_z),
            Velocity::default(),
        ))
        .id();

    let mut i = 0;
    for wheel in &robot.wheels {
        let revolute_joint = RevoluteJointBuilder::new(Vec3::Y)
            .local_anchor1(wheel.center + Vec3::new(0.0, 0.0, -robot.collider_z));
        // if wheel.motor {
        //     revolute_joint = revolute_joint.motor_velocity(1.0, 500.0);
        // }

        let mut pid = Pid::new(0.0, 0.01);
        pid.p(0.01, 0.01);

        commands.spawn((
            RigidBody::Dynamic,
            Collider::cylinder(wheel.thickness / 2.0, wheel.radius),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
            ImpulseJoint::new(new_robot, revolute_joint),
            Transform::from_xyz(
                wheel.center.x + robot_pos.x,
                wheel.center.y + robot_pos.y,
                wheel.center.z + robot_pos.z,
            ),
            Velocity::default(),
            ExternalForce::default(),
            Wheel(i, pid),
        ));

        i += 1;
    }

    for caster in &robot.casters {
        let spherical_joint = SphericalJointBuilder::new()
            .local_anchor1(caster.center + Vec3::new(0.0, 0.0, -robot.collider_z));

        commands.spawn((
            RigidBody::Dynamic,
            Collider::ball(caster.radius),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
            ImpulseJoint::new(new_robot, spherical_joint),
            Transform::from_xyz(
                caster.center.x + robot_pos.x,
                caster.center.y + robot_pos.y,
                caster.center.z + robot_pos.z,
            ),
            Velocity::default(),
        ));
    }

    for sensor in &robot.distance_sensors {
        let fixed_joint = FixedJointBuilder::new()
            .local_anchor1(sensor.center + Vec3::new(0.0, 0.0, -robot.collider_z))
            .local_basis1(Rot::from_rotation_arc(Vec3::X, sensor.facing));

        commands.spawn((
            RigidBody::Dynamic,
            Collider::ball(inches(0.15)),
            CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
            ImpulseJoint::new(new_robot, fixed_joint),
            Transform::from_xyz(
                sensor.center.x + robot_pos.x,
                sensor.center.y + robot_pos.y,
                sensor.center.z + robot_pos.z,
            ),
            GravityScale(0.0),
            Velocity::default(),
        ));
    }
}
