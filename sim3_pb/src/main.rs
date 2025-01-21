mod camera;
mod physics;

use crate::camera::{pan_orbit_camera, spawn_camera};
use crate::physics::spawn_walls;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use core_pb::grid::standard_grid::StandardGrid;
use pid::Pid;

#[derive(Resource)]
pub struct MyApp {
    standard_grid: StandardGrid,
}

#[derive(Component)]
pub struct Wall;

#[derive(Component)]
pub struct Wheel(usize, Pid<f32>);

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

fn keyboard_input(
    mut wheels: Query<(
        &mut ExternalForce,
        &Transform,
        &Velocity,
        &mut Wheel,
        Entity,
    )>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (mut ext_force, transform, velocity, mut wheel, _entity) in &mut wheels {
        // Is this the left wheel or the right wheel?
        let is_left_wheel = wheel.0 == 0;

        // Pick our forward/backward keys
        let (forward_key, backward_key) = if is_left_wheel {
            (KeyCode::KeyQ, KeyCode::KeyA)
        } else {
            (KeyCode::KeyE, KeyCode::KeyD)
        };

        // Decide the target speed (forward vs backward vs none)
        // You can also combine them if you want pressing forward/backward at once to sum or override.
        let mut speed = 0.0;
        if keys.pressed(forward_key) {
            speed = 6.0;
        } else if keys.pressed(backward_key) {
            speed = -6.0;
        }

        // --------------------------------
        // 1) Get the *local* angular velocity around this wheel's local axis
        //    Because velocity.angvel is a world-space vector.
        //    "transform.rotation.inverse() * velocity.angvel" = local space angvel
        // --------------------------------
        let local_angvel = transform.rotation.inverse() * velocity.angvel;
        // If the wheel rotates around its local Y axis, we use local_angvel.y
        let current_angvel = local_angvel.y;

        // Send the target setpoint (the desired wheel speed) to your PID controller
        wheel.1.setpoint(speed);
        let output = wheel.1.next_control_output(current_angvel);

        // --------------------------------
        // 2) Apply torque in world space around this wheel’s local Y axis
        //    = (world rotation) * (local axis)
        // --------------------------------
        let torque_axis = transform.rotation * Vec3::Y;
        ext_force.torque = torque_axis * output.output;
    }
}
