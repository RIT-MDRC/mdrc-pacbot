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

/// The number of guesses tracked by ParticleFilter
pub const NUM_PARTICLE_FILTER_POINTS: usize = 1000;
/// The number of rigid bodies tracked by the ParticleFilter
pub const NUM_PARTICLE_FILTER_BODIES: usize = 20;
/// The number of points displayed on the gui
pub const GUI_PARTICLE_FILTER_POINTS: usize = 1000;
/// The number of top guesses that are kept unchanged for the next generation
pub const PARTICLE_FILTER_ELITE: usize = 10;
/// The number of worst guesses that are deleted and randomly generated near the best guess
pub const PARTICLE_FILTER_PURGE: usize = 150;
/// The number of worst guesses that are deleted and randomly generated anywhere
pub const PARTICLE_FILTER_RANDOM: usize = 3;
