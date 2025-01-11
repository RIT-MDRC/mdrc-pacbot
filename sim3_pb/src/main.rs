mod camera;
mod physics;

use crate::camera::{pan_orbit_camera, spawn_camera};
use crate::physics::spawn_walls;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use core_pb::constants::GU_PER_M;
use core_pb::grid::standard_grid::StandardGrid;

#[derive(Resource)]
pub struct MyApp {
    standard_grid: StandardGrid,
}

#[derive(Component)]
pub struct Wall;

pub fn main() {
    info!("Simulation starting up");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(MyApp {
            standard_grid: StandardGrid::Pacman,
        })
        // .insert_resource(PacbotNetworkSimulation::new().unwrap())
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, pan_orbit_camera)
        // .add_systems(Update, keyboard_input)
        // .add_systems(Update, update_network)
        // .add_systems(Update, robot_position_to_game_state)
        .run();
}

fn setup_physics(
    app: ResMut<MyApp>,
    mut commands: Commands,
    mut config: Query<(Entity, &RapierContext, &mut RapierConfiguration)>,
) {
    spawn_walls(&mut commands, app.standard_grid);
}
