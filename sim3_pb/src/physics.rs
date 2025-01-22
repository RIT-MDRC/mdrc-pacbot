use crate::Wall;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use core_pb::constants::{INCHES_PER_GU, MM_PER_GU};
use core_pb::grid::standard_grid::StandardGrid;
use pid::Pid;

fn inches(x: f32) -> f32 {
    x / INCHES_PER_GU
}

#[derive(Copy, Clone, Debug)]
pub struct RobotShapeWheel {
    pub center: Vec3,
    pub radius: f32,
    pub thickness: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct RobotCasterWheel {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, Component)]
pub struct RobotDistanceSensor {
    pub center: Vec3,
    pub facing: Vec3,
}

#[derive(Clone, Debug)]
pub struct RobotShape {
    /// The dimensions of the rectangular collider that best represents the robot
    pub collider_size: Vec3,
    /// When the robot is stable on the ground, the height of the center of the main collider off the ground
    pub collider_z: f32,

    /// Description of the *motorized* wheels on this robot
    pub wheels: Vec<RobotShapeWheel>,
    /// Description of the distance sensors installed on the robot
    pub distance_sensors: Vec<RobotDistanceSensor>,

    /// Description of the *non-motorized* caster wheels on this robot
    pub casters: Vec<RobotCasterWheel>,
}

#[derive(Clone, Debug, Component)]
#[allow(dead_code)]
pub struct PhysicsRobot {
    /// Physical characteristics, measurements, etc.
    pub shape: RobotShape,

    /// The shape that collides with walls. Generally, its center can be considered the robot's position
    pub main_collider: Entity,
    /// Does not include non-motorized wheels
    pub motors: Vec<Entity>,
    /// These simulate motor controllers
    pub pids: Vec<Pid<f32>>,
    /// The center of sensors from which rays will be cast
    pub dists: Vec<Entity>,
    /// Placed at the end of rays cast from `dists` as a visual marker
    pub dist_raycast_markers: Vec<Entity>,

    /// All entities that are a part of this robot - useful for teleporting the entire robot
    pub associated_entities: Vec<Entity>,
}

pub fn update_robots(
    keys: Res<ButtonInput<KeyCode>>,
    rapier_context: ReadDefaultRapierContext,
    mut robots: Query<&mut PhysicsRobot>,
    mut t_v_f: Query<(&Transform, &Velocity, &mut ExternalForce)>,
    mut sensors: Query<(&mut Transform, &RobotDistanceSensor), Without<ExternalForce>>,
) {
    for mut robot in &mut robots {
        // update wheel PIDs
        for (w, motor_entity) in robot.motors.clone().iter().enumerate() {
            let (transform, velocity, mut ext_force) =
                t_v_f.get_mut(*motor_entity).expect("Motor wheel missing");

            let is_left_wheel = w == 0;

            let (forward_key, backward_key) = if is_left_wheel {
                (KeyCode::KeyQ, KeyCode::KeyA)
            } else {
                (KeyCode::KeyE, KeyCode::KeyD)
            };

            let mut speed = 0.0;
            if keys.pressed(forward_key) {
                speed = 6.0;
            } else if keys.pressed(backward_key) {
                speed = -6.0;
            }

            // Get the *local* angular velocity around this wheel's local axis
            let local_angvel = transform.rotation.inverse() * velocity.angvel;
            let current_angvel = local_angvel.y;

            // Send the target setpoint
            robot.pids[w].setpoint(speed);
            let output = robot.pids[w].next_control_output(current_angvel);

            // Apply torque in world space around this wheel’s local Y axis
            let torque_axis = transform.rotation * Vec3::Y;
            ext_force.torque = torque_axis * output.output;
        }
        // ray casts
        let filter: QueryFilter =
            QueryFilter::default().groups(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1));
        for (s, sensor_entity) in robot.dists.clone().iter().enumerate() {
            let (transform, _sensor) = sensors.get(*sensor_entity).expect("Sensor missing");

            let hit_point = rapier_context
                .cast_ray_and_get_normal(
                    transform.translation,
                    transform.rotation * Vec3::X,
                    8.0,
                    false,
                    filter,
                )
                .map(|(_, p)| p.point)
                .unwrap_or(transform.translation);

            let (mut marker_transform, _) = sensors
                .get_mut(robot.dist_raycast_markers[s])
                .expect("Marker missing");
            marker_transform.translation = hit_point;
        }
    }
}

impl PhysicsRobot {
    pub fn spawn(commands: &mut Commands, shape: RobotShape, pos: Vec3) -> Entity {
        let mut associated_entities = vec![];

        let pos_height = Vec3::ZERO.with_z(shape.collider_z);

        // Collider rectangle
        let main_collider = commands
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(
                    shape.collider_size.x / 2.0,
                    shape.collider_size.y / 2.0,
                    shape.collider_size.z / 2.0,
                ),
                CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                Transform::from_translation(pos + pos_height),
                Velocity::default(),
            ))
            .id();
        associated_entities.push(main_collider);

        // Wheels
        let motors = shape
            .wheels
            .iter()
            .map(|wheel| {
                let revolute_joint =
                    RevoluteJointBuilder::new(Vec3::Y).local_anchor1(wheel.center + -pos_height);

                let id = commands
                    .spawn((
                        RigidBody::Dynamic,
                        Collider::cylinder(wheel.thickness / 2.0, wheel.radius),
                        CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                        ImpulseJoint::new(main_collider, revolute_joint),
                        Transform::from_translation(wheel.center + pos),
                        Velocity::default(),
                        ExternalForce::default(),
                    ))
                    .id();

                associated_entities.push(id);
                id
            })
            .collect::<Vec<_>>();

        // Pids
        let pids = shape
            .wheels
            .iter()
            .map(|_| {
                let mut pid = Pid::new(0.0, 0.01);
                pid.p(0.01, 0.01);
                pid
            })
            .collect::<Vec<_>>();

        // Distance sensors
        let dists = shape
            .distance_sensors
            .iter()
            .map(|sensor| {
                let fixed_joint = FixedJointBuilder::new()
                    .local_anchor1(sensor.center + -pos_height)
                    .local_basis1(Rot::from_rotation_arc(Vec3::X, sensor.facing));

                let id = commands
                    .spawn((
                        RigidBody::Dynamic,
                        Collider::ball(inches(0.15)),
                        CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                        ImpulseJoint::new(main_collider, fixed_joint),
                        Transform::from_translation(sensor.center + pos),
                        GravityScale(0.0),
                        Velocity::default(),
                        *sensor,
                    ))
                    .id();

                associated_entities.push(id);
                id
            })
            .collect::<Vec<_>>();

        // Raycast markers
        let dist_raycast_markers = shape
            .distance_sensors
            .iter()
            .map(|sensor| {
                let id = commands
                    .spawn((
                        RigidBody::Fixed,
                        Collider::ball(inches(0.15)),
                        CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                        Transform::default(),
                        *sensor,
                    ))
                    .id();

                associated_entities.push(id);
                id
            })
            .collect::<Vec<_>>();

        // Non-motorized caster wheels
        for caster in &shape.casters {
            let spherical_joint =
                SphericalJointBuilder::new().local_anchor1(caster.center + -pos_height);

            let id = commands
                .spawn((
                    RigidBody::Dynamic,
                    Collider::ball(caster.radius),
                    CollisionGroups::new(Group::GROUP_2, Group::GROUP_1),
                    ImpulseJoint::new(main_collider, spherical_joint),
                    Transform::from_translation(caster.center + pos),
                    Velocity::default(),
                ))
                .id();

            associated_entities.push(id);
        }

        commands
            .spawn(Self {
                shape,

                main_collider,
                motors,
                pids,
                dists,
                dist_raycast_markers,

                associated_entities,
            })
            .id()
    }
}

pub fn spawn_walls(commands: &mut Commands, grid: StandardGrid) {
    let grid = grid.compute_grid();

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
            // .insert(ColliderDebug::NeverRender)
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
            },
            RobotShapeWheel {
                center: Vec3::new(
                    inches(0.77),
                    -(inches(1.7) + wheel_thickness / 2.0 + inches(0.05)),
                    wheel_radius,
                ),
                radius: wheel_radius,
                thickness: wheel_thickness,
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

    PhysicsRobot::spawn(commands, robot, robot_pos);
}
