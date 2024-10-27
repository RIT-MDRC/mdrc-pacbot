use crate::driving::SimRobot;
use crate::{MyApp, Robot, Wall};
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy_rapier2d::geometry::{Collider, CollisionGroups, Group};
use bevy_rapier2d::na::{Point2, Rotation2, Vector2};
use bevy_rapier2d::prelude::*;
use core::f32;
use core_pb::constants::GU_PER_M;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::RobotName;
use core_pb::robot_definition::RobotDefinition;
use rand::prelude::*;

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
            .insert(Collider::ball(name.robot().radius))
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

        let sim_robot = SimRobot::start(name, false, self.from_robots.0.clone());

        self.robots[name as usize] = Some((new_robot, sim_robot));
    }

    pub fn teleport_robot(
        &mut self,
        name: RobotName,
        loc: Point2<i8>,
        pos_query: &mut Query<&mut Transform>,
    ) {
        if let Some((entity, _)) = &self.robots[name as usize] {
            if let Ok(mut t) = pos_query.get_mut(*entity) {
                t.translation.x = loc.x as f32;
                t.translation.y = loc.y as f32;
            }
        }
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
        rapier_context: Res<RapierContext>,
    ) {
        for (_, t, v, mut imp, robot) in robots {
            // update simulated imu
            if let Some((_, sim_robot)) = &mut self.robots[robot.name as usize] {
                let rotation =
                    Rotation2::new(2.0 * t.rotation.normalize().w.acos() * t.rotation.z.signum())
                        .angle();
                sim_robot.write().unwrap().imu_angle = Ok(rotation);
                sim_robot.write().unwrap().velocity = v.linvel.into();
                sim_robot.write().unwrap().ang_velocity = v.angvel.into();

                let mut distance_sensors: [Result<Option<f32>, ()>; 4] = [Err(()); 4];

                for (i, _) in distance_sensors.into_iter().enumerate() {
                    let ray_pos = Vec2::new(
                        t.translation.x
                            + f32::cos(rotation + (i as f32) * f32::consts::FRAC_PI_2)
                                * robot.name.robot().radius,
                        t.translation.y
                            + f32::sin(rotation + (i as f32) * f32::consts::FRAC_PI_2)
                                * robot.name.robot().radius,
                    );
                    let ray_dir: Vec2 = Vec2::new(
                        f32::cos(rotation + (i as f32) * f32::consts::FRAC_PI_2),
                        f32::sin(rotation + (i as f32) * f32::consts::FRAC_PI_2),
                    );
                    let max_toi: f32 = RobotDefinition::new(robot.name).sensor_distance * GU_PER_M; // TODO: find actual sensor range (unit: space between pellets)
                    let solid: bool = true;
                    let filter: QueryFilter = QueryFilter::default()
                        .groups(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1));
                    if let Some((_, intersection)) = rapier_context
                        .cast_ray_and_get_normal(ray_pos, ray_dir, max_toi, solid, filter)
                    {
                        let hit_point = intersection.point;
                        let distance = ray_pos.distance(hit_point);

                        distance_sensors[i] = Ok(Some(distance));
                    } else {
                        distance_sensors[i] = Ok(None);
                    }
                }

                sim_robot.write().unwrap().distance_sensors = distance_sensors;

                let mut rng = thread_rng();
                let noise_rng: f32 = 0.08;
                let mut motor_speeds = robot.wasd_target_vel.unwrap_or([0.0; 3]);
                let robot_definition = RobotDefinition::new(robot.name);
                //for each motor add noise
                for i in 0..3 {
                    let noise: f32 = rng.gen_range(-noise_rng..noise_rng).abs();
                    if motor_speeds[i] != 0.0 {
                        motor_speeds[i] += motor_speeds[i] * noise;
                    }
                }
                sim_robot.write().unwrap().actual_motor_speeds = motor_speeds;
                let mut target_vel = robot_definition
                    .drive_system
                    .get_actual_vel_omni(motor_speeds);
                let move_scale = target_vel.0.magnitude();
                if target_vel.0 != Vector2::new(0.0, 0.0) {
                    target_vel.0 = target_vel.0.normalize() * move_scale;
                }
                imp.impulse.x = target_vel.0.x - v.linvel.x * 0.6;
                imp.impulse.y = target_vel.0.y - v.linvel.y * 0.6;
                imp.torque_impulse = target_vel.1 - v.angvel * 0.1;
            }
        }
    }

    pub fn reset_grid(
        &mut self,
        walls: &Query<(Entity, &Wall)>,
        robots: &mut Query<(
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut ExternalImpulse,
            &mut Robot,
        )>,
        commands: &mut Commands,
    ) {
        for wall in walls {
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
