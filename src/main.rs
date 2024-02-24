use crate::grid::standard_grids::StandardGrid;
use crate::grid::ComputedGrid;
use crate::gui::game::update_game;
use crate::gui::{font_setup, ui_system, AppMode, GuiApp, GuiStopwatch};
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
use crate::replay_manager::{replay_playback, update_replay_manager_system, ReplayManager};
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
    pub mode: AppMode,
    pub enable_ai: bool,
    pub pico_address: Option<String>,
    pub go_server_address: Option<String>,
    pub robot: Robot,

    pub replay_save_location: bool,
    pub replay_save_sensors: bool,
    pub replay_save_targets: bool,

    pub enable_pf: bool,
    /// The number of guesses tracked by ParticleFilter
    pub pf_total_points: usize,
    /// The number of points displayed on the gui
    pub pf_gui_points: usize,
    /// The number of top guesses that are kept unchanged for the next generation
    pub pf_elite: usize,
    /// The number of worst guesses that are deleted and randomly generated near the best guess
    pub pf_purge: usize,
    /// The number of worst guesses that are deleted and randomly generated anywhere
    pub pf_random: usize,

    pub pf_spread: f32,
    pub pf_elitism_bias: f32,
    pub pf_genetic_translation_limit: f32,
    pub pf_genetic_rotation_limit: f32,
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
            pf_elite: 10,
            pf_purge: 100,
            pf_random: 200,

            pf_spread: 2.5,
            pf_elitism_bias: 1.0,
            pf_genetic_translation_limit: 0.1,
            pf_genetic_rotation_limit: 0.1,
        }
    }
}

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
        .add_plugins(HLPlugin)
        .init_resource::<PacbotSensors>()
        .init_resource::<PacbotSensorsRecvTime>()
        .init_resource::<LightPhysicsInfo>()
        .init_resource::<PacbotSimulation>()
        .init_resource::<ComputedGrid>()
        .init_resource::<GuiApp>()
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
        .insert_resource(GuiStopwatch(Stopwatch::new(
            10,
            "GUI".to_string(),
            1.0,
            2.0,
        )))
        .insert_resource(ScheduleStopwatch(Stopwatch::new(
            10,
            "Schedule".to_string(),
            5.0,
            7.0,
        )))
        .add_systems(Startup, font_setup)
        .add_systems(PreUpdate, start_schedule_stopwatch)
        .add_systems(PostUpdate, end_schedule_stopwatch)
        .add_systems(
            Update,
            (
                // General
                update_game,
                target_path_to_target_vel,
                // Ui
                ui_system,
                update_replay_manager_system,
                replay_playback,
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
