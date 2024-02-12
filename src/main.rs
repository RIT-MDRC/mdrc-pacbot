use crate::grid::standard_grids::StandardGrid;
use crate::gui::game::update_game;
use crate::gui::{font_setup, ui_system, AppMode};
use crate::high_level::run_high_level;
use crate::network::NetworkPlugin;
use crate::pathing::target_path_to_target_vel;
use crate::physics::PhysicsPlugin;
use crate::replay_manager::{replay_playback, update_replay_manager_system};
use crate::robot::Robot;
use bevy::prelude::*;
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

            replay_save_location: true,
            replay_save_sensors: true,
            replay_save_targets: true,

            enable_pf: true,
            pf_total_points: 1000,
            pf_gui_points: 1000,
            pf_elite: 10,
            pf_purge: 150,
            pf_random: 50,

            pf_spread: 2.5,
            pf_elitism_bias: 1.0,
            pf_genetic_translation_limit: 0.1,
            pf_genetic_rotation_limit: 0.1,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins((NetworkPlugin, PhysicsPlugin))
        .add_systems(Startup, font_setup)
        .add_systems(
            Update,
            (
                run_high_level,
                ui_system,
                update_replay_manager_system,
                replay_playback,
                update_game,
                target_path_to_target_vel,
            ),
        )
        .run();
}
