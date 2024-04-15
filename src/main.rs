//! Utilities for writing blazingly fast Pacbot code

#![warn(missing_docs)]

use crate::grid::standard_grids::StandardGrid;
use crate::grid::{ComputedGrid, IntLocation};
use crate::gui::game::update_game;
use crate::gui::{ui_system, AppMode, GuiPlugin};
use crate::high_level::HLPlugin;
use crate::network::{
    reconnect_pico, recv_pico, send_motor_commands, LastMotorCommands, MotorRequest,
    NetworkPluginData, PacbotSensors, PacbotSensorsRecvTime,
};
use crate::pathing::{
    target_path_to_target_vel, test_path_position_to_target_path, TargetPath, TargetVelocity,
};
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
use network::{poll_gs, GameServerConn};
use pacbot_rs::game_engine::GameEngine;
use pathing::{create_test_path_target, GridSampleProbs};

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

/// Determines what is used to choose the destination and path
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum HighLevelStrategy {
    /// WASD, or right click to set target
    Manual,
    /// AI
    ReinforcementLearning,
    /// Test (random, uniform over all cells)
    TestUniform,
    /// Test (random, prefer non-explored cells)
    TestNonExplored,
}

/// Options that the user can set via the GUI, shared between most processes
#[derive(Resource)]
pub struct UserSettings {
    /// Whether the app is recording (normal) or playback for a replay
    pub mode: AppMode,
    /// Whether AI actions should be calculated
    pub high_level_strategy: HighLevelStrategy,
    /// Optional IP for the pico
    pub pico_address: Option<String>,
    /// Optional IP for the game server
    pub go_server_address: Option<String>,
    /// Physical characteristics of the robot
    pub robot: Robot,
    /// Whether sensor values should come from the robot, versus rapier
    pub sensors_from_robot: bool,
    /// When giving motor commands to the robot, should they be adjusted with the particle
    /// filter's current rotation?
    pub motors_ignore_phys_angle: bool,
    /// Non-PID pwm control, usually for testing configuration
    pub pwm_override: Option<[MotorRequest; 3]>,

    /// When the user left-clicks on a location where the simulated robot should be teleported
    pub kidnap_position: Option<IntLocation>,
    /// When the user right-clicks on a location that should be used as a target location
    pub test_path_position: Option<IntLocation>,

    /// Whether physical location should be saved in the replay
    pub replay_save_location: bool,
    /// Whether pacbot sensors should be saved in the replay
    pub replay_save_sensors: bool,
    /// Whether target paths and velocities should be saved in the replay
    pub replay_save_targets: bool,

    /// Whether particle filter is calculated
    pub enable_pf: bool,
    /// The number of guesses tracked by ParticleFilter
    pub pf_total_points: usize,
    /// The number of points displayed on the gui
    pub pf_gui_points: usize,
    /// All points with a larger error are removed
    pub pf_error_threshold: f32,
    /// Chance 0.0-1.0 that a new point will spawn near an existing one instead of randomly
    pub pf_chance_near_other: f32,
    /// The average number of times the robot is kidnapped per second, in our theoretical motion
    /// model. This determines the probability that a particle will be teleported to a random
    /// position.
    pub pf_avg_kidnaps_per_sec: f32,
    /// The standard deviation of the CV position error, in our theoretical sensor model.
    pub pf_cv_error_std: f32,

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
            high_level_strategy: HighLevelStrategy::Manual,
            pico_address: Some("192.168.4.209:20002".to_string()),
            go_server_address: None,
            robot: Robot::default(),
            sensors_from_robot: false,
            motors_ignore_phys_angle: true,
            pwm_override: None,

            kidnap_position: None,
            test_path_position: None,

            replay_save_location: false,
            replay_save_sensors: false,
            replay_save_targets: false,

            enable_pf: false,
            pf_total_points: 50000,
            pf_gui_points: 10000,
            pf_error_threshold: 2.0,
            pf_chance_near_other: 0.99,
            pf_avg_kidnaps_per_sec: 50.0,
            pf_cv_error_std: 5.0,

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
                present_mode: PresentMode::Fifo, //PresentMode::Immediate,
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
        .init_resource::<GridSampleProbs>()
        .init_resource::<TargetVelocity>()
        .init_resource::<ReplayManager>()
        .init_resource::<LastMotorCommands>()
        .init_non_send_resource::<GameServerConn>()
        .insert_resource(PhysicsStopwatch(Stopwatch::new(
            10,
            "Physics".to_string(),
            1.0,
            2.0,
        )))
        .insert_resource(ParticleFilterStopwatch(Stopwatch::new(
            10,
            "PF".to_string(),
            15.0,
            20.0,
        )))
        .insert_resource(ScheduleStopwatch(Stopwatch::new(
            10,
            "Schedule".to_string(),
            15.0,
            20.0,
        )))
        .add_systems(PreUpdate, start_schedule_stopwatch)
        .add_systems(PostUpdate, end_schedule_stopwatch)
        .add_systems(
            Update,
            (
                // General
                update_game,
                target_path_to_target_vel,
                test_path_position_to_target_path,
                create_test_path_target,
                // Networking
                reconnect_pico,
                send_motor_commands.after(reconnect_pico),
                recv_pico.after(reconnect_pico),
                poll_gs,
                // Physics
                run_simulation.after(ui_system),
                run_particle_filter.after(run_simulation),
                update_physics_info.after(run_particle_filter),
                update_game_state_pacbot_loc.after(update_physics_info),
            ),
        )
        .run();
}
