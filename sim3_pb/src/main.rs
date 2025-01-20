mod camera;
mod physics;

use crate::camera::{pan_orbit_camera, spawn_camera};
use crate::physics::spawn_walls;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
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
        .add_systems(Startup, spawn_camera)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, pan_orbit_camera)
        .add_systems(Update, keyboard_input)
        .run();
}

fn setup_physics(app: ResMut<MyApp>, mut commands: Commands) {
    spawn_walls(&mut commands, app.standard_grid);
}

fn keyboard_input() {}
