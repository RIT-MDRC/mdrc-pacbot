//! Utilities for writing blazingly fast Pacbot code

#![warn(missing_docs)]

use crate::grid::standard_grids::StandardGrid;
use crate::grid::ComputedGrid;
use crate::gui::game::update_game;
use crate::gui::{ui_system, AppMode, GuiPlugin};
use crate::high_level::HLPlugin;
use crate::network::{
    reconnect_pico, recv_pico, send_motor_commands, NetworkPluginData, PacbotSensors,
    PacbotSensorsRecvTime,
};
use crate::pathing::{target_path_to_target_vel, TargetPath, TargetVelocity};
use crate::physics::{
    run_particle_filter, run_simulation, update_game_state_pacbot_loc, update_physics_info,
    LightPhysicsInfo, PacbotSimulation, ParticleFilterStopwatch, PhysicsStopwatch,
};
use crate::replay_manager::ReplayManager;
use crate::robot::Robot;
use crate::util::stopwatch::Stopwatch;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use pacbot_rs::game_engine::GameEngine;

pub mod grid;
pub mod gui;
pub mod physics;
pub mod util;

mod high_level;
pub mod network;
mod pathing;
pub mod replay;
mod replay_manager;
pub mod robot;

/// The state of Pacman, the game
#[derive(Default, Resource)]
pub struct PacmanGameState(GameEngine);

/// The current StandardGrid, which determines the shape of the walls
#[derive(Default, Resource)]
pub struct StandardGridResource(StandardGrid);

/// Options that the user can set via the GUI, shared between most processes
#[derive(Resource)]
pub struct UserSettings {
    /// Whether the app is recording (normal) or playback for a replay
    pub mode: AppMode,
    /// Whether AI actions should be calculatede
    pub enable_ai: bool,
    /// Optional IP for the pico
    pub pico_address: Option<String>,
    /// Optional IP for the game server
    pub go_server_address: Option<String>,
    /// Physical characteristics of the robot
    pub robot: Robot,

    /// Whether physical location should be saved in the replay
    pub replay_save_location: bool,
    /// Whether pacbot sensors should be saved in the replay
    pub replay_save_sensors: bool,
    /// Whether target paths and velocities should be saved in the replay
    pub replay_save_targets: bool,

    /// Currently always true
    pub enable_pf: bool,
    /// The number of guesses tracked by ParticleFilter
    pub pf_total_points: usize,
    /// The number of points displayed on the gui
    pub pf_gui_points: usize,
    /// All points with a larger error are removed
    pub pf_error_threshold: f32,
    /// Chance 0.0-1.0 that a new point will spawn near an existing one instead of randomly
    pub pf_chance_near_other: f32,

    /// When generating a point based on an existing point, how far can it be moved in x and y?
    pub pf_translation_limit: f32,
    /// When generating a point based on an existing point, how far can it be moved in rotation?
    pub pf_rotation_limit: f32,

    /// When moving particles by Rapier-reported distance, add noise proportional to translation
    pub pf_simulated_translation_noise: f32,
    /// When moving particles by Rapier-reported distance, add noise proportional to rotation
    pub pf_simulated_rotation_noise: f32,
    /// When moving particles by Rapier-reported distance, add noise
    pub pf_generic_noise: f32,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            mode: AppMode::Recording,
            enable_ai: false,
            pico_address: None,
            go_server_address: None,
            robot: Robot::default(),

            replay_save_location: false,
            replay_save_sensors: false,
            replay_save_targets: false,

            enable_pf: true,
            pf_total_points: 1000,
            pf_gui_points: 1000,
            pf_error_threshold: 2.0,
            pf_chance_near_other: 0.99,

            pf_translation_limit: 0.3,
            pf_rotation_limit: 0.3,

            pf_simulated_translation_noise: 0.03,
            pf_simulated_rotation_noise: 0.02,
            pf_generic_noise: 0.02,
        }
    }
}

/// Keeps track of how long it takes to run the whole schedule
#[derive(Resource)]
pub struct ScheduleStopwatch(Stopwatch);

fn start_schedule_stopwatch(mut stopwatch: ResMut<ScheduleStopwatch>) {
    stopwatch.0.start();
}

fn end_schedule_stopwatch(mut stopwatch: ResMut<ScheduleStopwatch>) {
    stopwatch.0.mark_segment("Update Finish");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .add_plugins((HLPlugin, GuiPlugin))
        .init_resource::<PacbotSensors>()
        .init_resource::<PacbotSensorsRecvTime>()
        .init_resource::<LightPhysicsInfo>()
        .init_resource::<PacbotSimulation>()
        .init_resource::<ComputedGrid>()
        .init_resource::<PacmanGameState>()
        .init_resource::<StandardGridResource>()
        .init_resource::<UserSettings>()
        .init_resource::<NetworkPluginData>()
        .init_resource::<TargetPath>()
        .init_resource::<TargetVelocity>()
        .init_resource::<ReplayManager>()
        .insert_resource(PhysicsStopwatch(Stopwatch::new(
            10,
            "Physics".to_string(),
            4.0,
            6.0,
        )))
        .insert_resource(ParticleFilterStopwatch(Stopwatch::new(
            10,
            "PF".to_string(),
            4.0,
            6.0,
        )))
        .insert_resource(ScheduleStopwatch(Stopwatch::new(
            10,
            "Schedule".to_string(),
            5.0,
            7.0,
        )))
        .add_systems(PreUpdate, start_schedule_stopwatch)
        .add_systems(PostUpdate, end_schedule_stopwatch)
        .add_systems(
            Update,
            (
                // General
                update_game,
                target_path_to_target_vel,
                // Networking
                reconnect_pico,
                send_motor_commands.after(reconnect_pico),
                recv_pico.after(reconnect_pico),
                // Physics
                run_simulation.after(ui_system),
                run_particle_filter.after(run_simulation),
                update_physics_info.after(run_particle_filter),
                update_game_state_pacbot_loc.after(update_physics_info),
            ),
        )
        .run();
}
