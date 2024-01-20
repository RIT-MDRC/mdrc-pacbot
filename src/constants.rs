//! Provides constants for the library.

/// Number of lives Pacman starts with
pub const STARTING_LIVES: u8 = 3;
/// Number of frames Pacman is invincible after eating a power pellet
pub const FRIGHTENED_LENGTH: u8 = 40;
/// Score for eating a pellet
pub const PELLET_SCORE: usize = 10;
/// Score for eating a power pellet
pub const POWER_PELLET_SCORE: usize = 50;
/// Score for eating a ghost
pub const GHOST_SCORE: usize = 200;
/// Score for eating a cherry
pub const CHERRY_SCORE: usize = 100;