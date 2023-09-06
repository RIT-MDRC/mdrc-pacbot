//! Structs to define the state of a game of Pacman
use crate::agent_setup::PacmanAgentSetup;
use crate::constants::STARTING_LIVES;
use crate::grid::{Direction, GridValue};
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
    /// Ghost behavior to resume when the game is un paused
    mode_on_resume: GhostMode,

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

impl PacmanState {
    /// Create a new PacmanState from a PacmanAgentSetup
    pub fn new(agent_setup: &PacmanAgentSetup) -> Self {
        let mut s = Self {
            mode: GhostMode::Paused,
            score: 0,
            lives: 0,
            elapsed_time: 0,
            pacman: Agent {
                location: Default::default(),
                direction: Direction::Right,
            },
            pellets: vec![],
            power_pellets: vec![],
            mode_on_resume: GhostMode::Chase,
        };

        s.reset(agent_setup);

        s
    }

    /// Reset the game state to the initial state using the same or different PacmanAgentSetup
    pub fn reset(&mut self, agent_setup: &PacmanAgentSetup) {
        self.mode = GhostMode::Paused;
        self.mode_on_resume = GhostMode::Chase;

        self.score = 0;
        self.lives = STARTING_LIVES;
        self.elapsed_time = 0;

        self.pacman = Agent {
            location: agent_setup.pacman_start().0,
            direction: agent_setup.pacman_start().1,
        };

        self.pellets = Vec::new();
        self.power_pellets = Vec::new();
        for p in agent_setup.grid().walkable_nodes() {
            let grid_value = agent_setup.grid().at(p).unwrap();
            self.pellets.push(grid_value == GridValue::o);

            if grid_value == GridValue::O {
                self.power_pellets.push(p.to_owned());
            }
        }
    }

    /// Update Pacman's location and direction
    pub fn update_pacman(&mut self, p: Point2<u8>, d: Direction) {
        self.pacman = Agent {
            location: p,
            direction: d,
        };
    }

    /// Pause the game
    pub fn pause(&mut self) {
        if self.mode != GhostMode::Paused {
            self.mode_on_resume = self.mode;
            self.mode = GhostMode::Paused;
        }
    }

    /// Resume the game
    pub fn resume(&mut self) {
        if self.mode == GhostMode::Paused {
            self.mode = self.mode_on_resume;
        }
    }
}

impl Default for PacmanState {
    fn default() -> Self {
        PacmanState::new(&PacmanAgentSetup::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::game_state::PacmanState;

    #[test]
    fn default_game_setup() {
        PacmanState::default();
    }
}
