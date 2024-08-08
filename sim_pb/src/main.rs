use async_channel::{unbounded, Receiver, Sender};
use bevy::prelude::*;
use bevy_rapier2d::na::Vector2;
use bevy_rapier2d::prelude::*;
use std::sync::{Arc, RwLock};

use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::pacbot_rs::location::LocationState;

use crate::driving::SimRobot;
use crate::network::{update_network, PacbotNetworkSimulation};
use crate::physics::spawn_walls;

#[allow(dead_code)]
mod delayed_value;
mod driving;
mod network;
mod physics;

#[derive(Resource)]
#[allow(clippy::type_complexity)]
pub struct MyApp {
    standard_grid: StandardGrid,
    grid: ComputedGrid,

    server_target_vel: [Option<(Vector2<f32>, f32)>; NUM_ROBOT_NAMES],

    robots: [Option<(Entity, Arc<RwLock<SimRobot>>)>; NUM_ROBOT_NAMES],
    from_robots: (
        Sender<(RobotName, RobotToSimulationMessage)>,
        Receiver<(RobotName, RobotToSimulationMessage)>,
    ),
    selected_robot: RobotName,
}

#[derive(Copy, Clone, Debug)]
pub enum SimulationToRobotMessage {}

#[derive(Copy, Clone, Debug)]
pub enum RobotToSimulationMessage {
    SimulatedVelocity(Vector2<f32>, f32),
    MarkFirmwareUpdated,
    Reboot,
}

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Robot {
    name: RobotName,
    wasd_target_vel: Option<(Vector2<f32>, f32)>,
}

fn main() {
    println!("Simulation starting up");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(MyApp {
            standard_grid: StandardGrid::Pacman,
            grid: StandardGrid::Pacman.compute_grid(),

            server_target_vel: [None; NUM_ROBOT_NAMES],

            robots: RobotName::get_all().map(|_| None),
            from_robots: unbounded(),

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
    mut app: ResMut<MyApp>,
    mut commands: Commands,
    mut rapier_configuration: ResMut<RapierConfiguration>,
) {
    rapier_configuration.gravity = Vect::ZERO;

    spawn_walls(&mut commands, app.standard_grid);
    app.spawn_robot(&mut commands, RobotName::Stella);
}

fn robot_position_to_game_state(
    app: ResMut<MyApp>,
    mut network: ResMut<PacbotNetworkSimulation>,
    robots: Query<(Entity, &Transform, &Robot)>,
) {
    for robot in &robots {
        if robot.2.name == app.selected_robot {
            let pos = app
                .grid
                .node_nearest(robot.1.translation.x, robot.1.translation.y)
                .unwrap();
            let new_loc = LocationState {
                row: pos.x,
                col: pos.y,
                dir: network.game_state.pacman_loc.dir,
            };
            if network.game_state.pacman_loc != new_loc {
                network.game_state.set_pacman_location((pos.x, pos.y))
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
        &mut Robot,
    )>,
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
    for (_, _, _, _, mut robot) in &mut robots {
        let mut target_vel = (Vector2::new(0.0, 0.0), 0.0);
        for (key, dir) in &key_directions {
            if robot.name == app.selected_robot && keys.pressed(*key) {
                target_vel.0 += dir.0;
                target_vel.1 += dir.1;
            }
        }
        if target_vel == (Vector2::new(0.0, 0.0), 0.0) {
            robot.wasd_target_vel = app.server_target_vel[robot.name as usize];
        } else {
            robot.wasd_target_vel = Some(target_vel)
        }
    }
    app.apply_robots_target_vel(&mut robots);
    if keys.just_pressed(KeyCode::KeyG) {
        app.standard_grid = match app.standard_grid {
            StandardGrid::Pacman => StandardGrid::Playground,
            _ => StandardGrid::Pacman,
        };
        app.grid = app.standard_grid.compute_grid();
        app.reset_grid(walls, &mut robots, &mut commands)
    }
}
