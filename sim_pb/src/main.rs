use bevy::prelude::*;
use bevy_rapier2d::na::Vector2;
use bevy_rapier2d::prelude::*;
use std::sync::{Arc, RwLock};

use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};

use crate::driving::SimRobot;
use crate::network::{update_network, PacbotNetworkSimulation};
use crate::physics::spawn_walls;

#[allow(dead_code)]
mod delayed_value;
mod driving;
mod network;
mod physics;

#[derive(Resource)]
pub struct MyApp {
    standard_grid: StandardGrid,
    grid: ComputedGrid,

    robots: [Option<(Entity, Arc<RwLock<SimRobot>>)>; NUM_ROBOT_NAMES],
    selected_robot: RobotName,
}

#[derive(Clone, Component)]
pub struct RobotReference(RobotName, Arc<RwLock<SimRobot>>);

#[derive(Component)]
pub struct Wall;

fn main() {
    info!("Simulation starting up");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(MyApp {
            standard_grid: StandardGrid::Pacman,
            grid: StandardGrid::Pacman.compute_grid(),

            robots: RobotName::get_all().map(|_| None),
            selected_robot: RobotName::Stella,
        })
        .insert_resource(PacbotNetworkSimulation::new().unwrap())
        .add_systems(Startup, setup_graphics)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, keyboard_input)
        .add_systems(Update, update_network)
        .add_systems(Update, robot_position_to_game_state)
        .run();
}

fn setup_graphics(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    let mut camera = Camera2dBundle::default();

    camera.transform.translation = Vec3::new(15.5, 15.5, 0.0);
    camera.projection.scale = 0.05;

    commands.spawn(camera);
}

fn setup_physics(
    app: ResMut<MyApp>,
    mut commands: Commands,
    mut rapier_configuration: ResMut<RapierConfiguration>,
) {
    rapier_configuration.gravity = Vect::ZERO;

    spawn_walls(&mut commands, app.standard_grid);
}

fn robot_position_to_game_state(
    app: ResMut<MyApp>,
    mut network: ResMut<PacbotNetworkSimulation>,
    robots: Query<(Entity, &Transform, &RobotReference)>,
) {
    for (_, t, robot) in &robots {
        if robot.0 == app.selected_robot {
            if let Some(pos) = app.grid.node_nearest(t.translation.x, t.translation.y) {
                if !network
                    .game_state
                    .wall_at(network.game_state.pacman_loc.get_coords())
                    && network.game_state.pacman_loc.get_coords() != (pos.x, pos.y)
                {
                    network.game_state.set_pacman_location((pos.x, pos.y))
                }
            }
        }
    }
}

fn keyboard_input(
    mut app: ResMut<MyApp>,
    mut commands: Commands,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    walls: Query<(Entity, &Wall)>,
    mut robots: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &mut ExternalImpulse,
        &RobotReference,
    )>,
    rapier_context: Res<RapierContext>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(name) = RobotName::get_all()
            .into_iter()
            .find(|name| name.is_simulated() && app.robots[*name as usize].is_none())
        {
            app.spawn_robot(&mut commands, name);
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        let name = app.selected_robot;
        app.despawn_robot(name, &mut commands);
        keys.press(KeyCode::Tab);
    }
    if keys.just_pressed(KeyCode::Tab) {
        app.selected_robot = RobotName::get_all()
            .into_iter()
            .map(|x| ((x as usize + app.selected_robot as usize + 1) % NUM_ROBOT_NAMES).into())
            .find(|x| app.robots[*x as usize].is_some())
            .unwrap_or(RobotName::Stella);
    }
    let key_directions = [
        (KeyCode::KeyW, (Vector2::new(0.0, 1.0), 0.0)),
        (KeyCode::KeyA, (Vector2::new(-1.0, 0.0), 0.0)),
        (KeyCode::KeyD, (Vector2::new(1.0, 0.0), 0.0)),
        (KeyCode::KeyS, (Vector2::new(0.0, -1.0), 0.0)),
        (KeyCode::KeyQ, (Vector2::new(0.0, 0.0), 0.3)),
        (KeyCode::KeyE, (Vector2::new(0.0, 0.0), -0.3)),
    ];
    for (_, _, _, _, robot) in &robots {
        let mut target_vel = (Vector2::new(0.0, 0.0), 0.0);
        for (key, dir) in &key_directions {
            if robot.0 == app.selected_robot && keys.pressed(*key) {
                target_vel.0 += dir.0;
                target_vel.1 += dir.1;
            }
        }
        if target_vel != (Vector2::new(0.0, 0.0), 0.0) {
            let motors = robot
                .0
                .robot()
                .drive_system
                .get_motor_speed_omni(target_vel.0, target_vel.1);
            robot.1.write().unwrap().wasd_motor_speeds = Some(motors)
        }
    }

    app.apply_robots_target_vel(&mut robots, rapier_context);
    if keys.just_pressed(KeyCode::KeyG) {
        app.standard_grid = match app.standard_grid {
            StandardGrid::Pacman => StandardGrid::Playground,
            _ => StandardGrid::Pacman,
        };
        app.grid = app.standard_grid.compute_grid();
        app.reset_grid(&walls, &mut robots, &mut commands)
    }
}
