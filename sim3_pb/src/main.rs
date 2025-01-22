mod camera;
mod physics;

use crate::camera::{pan_orbit_camera, spawn_camera};
use crate::physics::{spawn_walls, update_robots};
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
        .add_systems(Update, update_robots)
        .run();
}

fn setup_physics(
    app: ResMut<MyApp>,
    mut commands: Commands,
    mut rapier: Query<&mut RapierConfiguration>,
) {
    for mut r in &mut rapier {
        r.gravity = -Vect::Z * 9.81;
    }

    spawn_walls(&mut commands, app.standard_grid);
}

// fn ray_casts(
//     rapier_context: ReadDefaultRapierContext,
//     sensors: Query<(&Transform, &DistanceSensor)>,
// ) {
//     let filter: QueryFilter =
//         QueryFilter::default().groups(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1));
//     for s in &sensors {
//         rapier_context.cast_ray_and_get_normal(s.0.translation, s.1 .0.facing, 8.0, true, filter);
//     }
// }
