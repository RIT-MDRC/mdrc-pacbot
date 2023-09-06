//! Structs to define the state of a game of Pacman
use crate::grid::Direction;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rapier2d::na::Point2;

/// Current ghost behavior - applies to all ghosts
///
/// When paused, Pacman should not move
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GhostMode {
    /// Ghosts are chasing Pacman
    Chase = 0,
    /// Ghosts are scattering to their respective corners
    Scatter = 1,
    /// Ghosts are frightened of Pacman
    Frightened = 2,
    /// The game is paused
    Paused = 3,
}

/// Ghost colors
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum GhostType {
    /// Directly chases Pacman
    Red = 2,
    /// Aims for 4 tiles in front of Pacman
    Pink = 3,
    /// Toggles between chasing Pacman and running away to his corner
    Orange = 4,
    /// Complicated behavior
    Blue = 5,
}

/// Information about a moving entity (Pacman or a ghost) during a game of Pacman
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Agent {
    /// The agent's current location in the [`Grid`]
    location: Point2<u8>,
    /// Current facing direction
    direction: Direction,
}

/// Information about a ghost during a game of Pacman
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ghost {
    /// Location and direction
    agent: Agent,
    /// Determines ghost behavior
    color: GhostType,
    /// If frightened, the amount of time remaining as frightened
    frightened_counter: Option<u8>,
}

/// Information that changes during a game of Pacman
///
/// Note: frightened_counter is not present because its only effect is Pacman's speed after collecting a power pellet
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PacmanState {
    /// Current ghost behavior - applies to all ghosts
    ///
    /// When paused, Pacman should not move
    mode: GhostMode,

    /// Player's current game score
    score: usize,
    /// Lives remaining - starts at 3; at 0, the game is over
    lives: u8,
    /// Number of frames that have passed since the start of the game
    elapsed_time: u32,

    /// Pacman's location and direction
    pacman: Agent,

    /// Pellets remaining
    pellets: Vec<bool>,
    /// Super pellets remaining
    power_pellets: Vec<Point2<u8>>,
}
