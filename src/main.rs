use crate::grid::standard_grids::StandardGrid;
use crate::gui::{font_setup, ui_system, AppMode};
use crate::high_level::run_high_level;
use crate::network::NetworkPlugin;
use crate::physics::PhysicsPlugin;
use crate::replay_manager::{replay_playback, update_replay_manager_system};
use bevy::prelude::*;
use pacbot_rs::game_engine::GameEngine;

pub mod grid;
pub mod gui;
pub mod physics;
pub mod util;

pub mod constants;
mod high_level;
pub mod network;
mod pathing;
pub mod replay;
mod replay_manager;
pub mod robot;

/// The state of Pacman, the game
#[derive(Default, Resource)]
pub struct PacmanGameState(GameEngine);

/// Options that the user can set via the GUI, shared between most processes
#[derive(Resource)]
pub struct UserSettings {
    pub mode: AppMode,
    pub enable_ai: bool,
    pub enable_pico: bool,
    pub pico_address: String,

    pub replay_save_location: bool,
    pub replay_save_sensors: bool,
    pub replay_save_targets: bool,
}

/// The current StandardGrid, which determines the shape of the walls
#[derive(Default, Resource)]
pub struct StandardGridResource(StandardGrid);

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
            ),
        )
        .run();
}
