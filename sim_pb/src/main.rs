#[allow(dead_code)]
mod delayed_value;
mod driving;
mod network;
mod physics;

use crate::network::{update_network, PacbotNetworkSimulation};
use crate::physics::spawn_walls;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::pacbot_rs::location::LocationState;

// todo
const ROBOT_RADIUS: f32 = 0.75;

#[derive(Resource)]
pub struct MyApp {
    grid: StandardGrid,

    robots: Vec<Entity>,
    selected_robot: Option<Entity>,
}

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Robot {
    wasd_target_vel: Option<(Vec2, f32)>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(MyApp {
            grid: StandardGrid::Pacman,

            robots: vec![],
            selected_robot: None,
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
    app: Res<MyApp>,
    mut commands: Commands,
    mut rapier_configuration: ResMut<RapierConfiguration>,
) {
    rapier_configuration.gravity = Vect::ZERO;

    spawn_walls(&mut commands, app.grid)
}

fn robot_position_to_game_state(
    app: ResMut<MyApp>,
    mut network: ResMut<PacbotNetworkSimulation>,
    robots: Query<(Entity, &Transform)>,
) {
    let grid = app.grid.compute_grid();
    if let Some(selected) = app.selected_robot {
        for robot in &robots {
            if robot.0 == selected {
                let pos = grid
                    .node_nearest(robot.1.translation.x, robot.1.translation.y)
                    .unwrap();
                let new_loc = LocationState {
                    row: pos.x,
                    col: pos.y,
                    dir: 0,
                };
                if network.game_state.pacman_loc != new_loc {
                    network.game_state.set_pacman_location(new_loc)
                }
            }
        }
    }
}

fn keyboard_input(
    mut app: ResMut<MyApp>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
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
        app.spawn_robot(&mut commands);
    }
    if keys.just_pressed(KeyCode::Tab) {
        if let Some(selected) = app.selected_robot {
            let index = app.robots.iter().position(|x| *x == selected).unwrap();
            let index = (index + 1) % app.robots.len();
            app.selected_robot = Some(app.robots[index])
        } else if let Some(selected) = app.robots.first() {
            app.selected_robot = Some(*selected)
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        if let Some(selected) = app.selected_robot {
            app.despawn_robot(selected, &mut commands);
            app.selected_robot = None;
            app.robots.retain(|x| *x != selected)
        }
    }
    let key_directions = [
        (KeyCode::KeyW, (Vec2::new(0.0, 1.0), 0.0)),
        (KeyCode::KeyA, (Vec2::new(-1.0, 0.0), 0.0)),
        (KeyCode::KeyD, (Vec2::new(1.0, 0.0), 0.0)),
        (KeyCode::KeyS, (Vec2::new(0.0, -1.0), 0.0)),
        (KeyCode::KeyQ, (Vec2::new(0.0, 0.0), 0.3)),
        (KeyCode::KeyE, (Vec2::new(0.0, 0.0), -0.3)),
    ];
    for (e, _, _, _, mut robot) in &mut robots {
        let mut target_vel = (Vec2::ZERO, 0.0);
        if let Some(selected) = app.selected_robot {
            for (key, dir) in &key_directions {
                if e == selected && keys.pressed(*key) {
                    target_vel.0 += dir.0;
                    target_vel.1 += dir.1;
                }
            }
        }
        if target_vel == (Vec2::ZERO, 0.0) {
            robot.wasd_target_vel = None
        } else {
            robot.wasd_target_vel = Some(target_vel)
        }
    }
    app.apply_robots_target_vel(&mut robots);
    if keys.just_pressed(KeyCode::KeyG) {
        app.grid = match app.grid {
            StandardGrid::Pacman => StandardGrid::Playground,
            _ => StandardGrid::Pacman,
        };
        app.reset_grid(walls, &mut robots, &mut commands)
    }
}
