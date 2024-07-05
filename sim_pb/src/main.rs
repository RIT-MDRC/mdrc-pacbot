use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use core_pb::grid::standard_grid::StandardGrid;

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
pub struct Robot;

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
        .add_systems(Startup, setup_graphics)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, keyboard_input)
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
    commands: Commands,
    mut rapier_configuration: ResMut<RapierConfiguration>,
) {
    rapier_configuration.gravity = Vect::ZERO;

    spawn_walls(commands, app.grid)
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
        &Robot,
    )>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        let pos = app.grid.get_default_pacbot_isometry().translation;

        let new_robot = commands
            .spawn(RigidBody::Dynamic)
            .insert(Collider::ball(ROBOT_RADIUS))
            .insert(CollisionGroups::new(Group::GROUP_2, Group::GROUP_1))
            .insert(TransformBundle::from(Transform::from_xyz(
                pos.x, pos.y, 0.0,
            )))
            .insert(ExternalImpulse::default())
            .insert(Velocity::default())
            .insert(Robot)
            .id();

        app.robots.push(new_robot);
        app.selected_robot = Some(new_robot);
    }
    if keys.just_pressed(KeyCode::Tab) {
        if let Some(selected) = app.selected_robot {
            let index = app.robots.iter().position(|x| *x == selected).unwrap();
            let index = (index + 1) % app.robots.len();
            app.selected_robot = Some(app.robots[index])
        } else if let Some(selected) = app.robots.get(0) {
            app.selected_robot = Some(*selected)
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        if let Some(selected) = app.selected_robot {
            commands.entity(selected).despawn();
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
    for (e, _, v, mut imp, _) in &mut robots {
        let mut target_vel = (Vec2::ZERO, 0.0);
        if let Some(selected) = app.selected_robot {
            for (key, dir) in &key_directions {
                if keys.pressed(*key) {
                    if e == selected {
                        target_vel.0 += dir.0;
                        target_vel.1 += dir.1;
                    }
                }
            }
        }
        let move_scale = 4.0;
        if target_vel.0 != Vec2::ZERO {
            target_vel.0 = target_vel.0.normalize() * move_scale;
        }
        imp.impulse = target_vel.0 - v.linvel * 0.6;
        imp.torque_impulse = target_vel.1 - v.angvel * 0.1;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        app.grid = match app.grid {
            StandardGrid::Pacman => StandardGrid::Playground,
            _ => StandardGrid::Pacman,
        };
        for wall in &walls {
            commands.entity(wall.0).despawn()
        }
        spawn_walls(commands, app.grid);
        for (_, mut t, mut v, _, _) in &mut robots {
            let pos = app.grid.get_default_pacbot_isometry().translation;
            t.translation = Vec3::new(pos.x, pos.y, 0.0);
            v.linvel = Vect::ZERO;
            v.angvel = 0.0;
        }
    }
}

fn spawn_walls(mut commands: Commands, grid: StandardGrid) {
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
