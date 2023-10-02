//! Static information needed to set up a Pacman game
use crate::game_state::GhostType;
use crate::grid::GridValue::{o, O};
use crate::grid::{ComputedGrid, Direction, GridValue};
use anyhow::{anyhow, Error};
use rapier2d::na::Point2;

/// Static information needed to set up a ghost for Pacman game
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GhostSetup {
    /// The ghost's starting path; where it goes when it first spawns
    pub start_path: Vec<(Point2<u8>, Direction)>,
    /// The ghost's color; determines behavior
    pub color: GhostType,
    /// The ghost's scatter point; where it goes when it's not chasing Pacman
    pub scatter_point: Point2<u8>,
}

/// Static information needed to set up a Pacman game
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PacmanAgentSetup {
    /// The grid
    grid: ComputedGrid,
    /// The Pacman start point
    pacman_start: (Point2<u8>, Direction),
    /// The ghosts
    ghosts: Vec<GhostSetup>,
    /// The times when ghosts swap between chase and scatter mode
    state_swap_times: Vec<u32>,
    /// The path that ghosts take upon respawning
    ghost_respawn_path: Vec<(Point2<u8>, Direction)>,
    /// This is the location where a ghost will reappear after being eaten by Pacman.
    ghost_home_pos: (Point2<u8>, Direction),
}

impl PacmanAgentSetup {
    /// Create a new PacmanGridSetup from a grid and a list of ghost start points
    pub fn new(
        grid: ComputedGrid,
        pacman_start: (Point2<u8>, Direction),
        ghosts: Vec<GhostSetup>,
        state_swap_times: Vec<u32>,
        ghost_respawn_path: Vec<(Point2<u8>, Direction)>,
    ) -> Result<Self, Error> {
        let start_value = grid
            .at(&pacman_start.0)
            .ok_or(anyhow!("Pacman start position doesn't exist"))?;
        if !start_value.walkable() {
            return Err(anyhow!("Pacman start position is not walkable"));
        }
        if start_value == o || start_value == O {
            return Err(anyhow!(
                "Pacman start position should not be a pellet or power pellet"
            ));
        }

        for ghost in &ghosts {
            if ghost.start_path.is_empty() {
                return Err(anyhow!("Ghost start path is empty"));
            }

            if grid
                .at(&ghost.scatter_point)
                .ok_or(anyhow!("Ghost path position doesn't exist"))?
                == GridValue::I
            {
                return Err(anyhow!("Ghost scatter point is not walkable"));
            }
            for point in &ghost.start_path {
                if grid
                    .at(&point.0)
                    .ok_or(anyhow!("Ghost path position doesn't exist"))?
                    == GridValue::I
                {
                    return Err(anyhow!("Ghost start path is not walkable"));
                }
            }
        }

        Ok(Self {
            grid,
            pacman_start,
            ghosts,
            state_swap_times,
            ghost_respawn_path,
            ghost_home_pos: (Point2::new(12, 15), Direction::Up),
        })
    }

    /// Get the grid
    pub fn grid(&self) -> &ComputedGrid {
        &self.grid
    }

    /// Get the Pacman start point
    pub fn pacman_start(&self) -> &(Point2<u8>, Direction) {
        &self.pacman_start
    }

    /// Get the ghosts
    pub fn ghosts(&self) -> &Vec<GhostSetup> {
        &self.ghosts
    }

    /// Get the times when ghosts swap between chase and scatter mode
    pub fn state_swap_times(&self) -> &Vec<u32> {
        &self.state_swap_times
    }

    /// Get the path that ghosts take upon respawning
    pub fn ghost_respawn_path(&self) -> &Vec<(Point2<u8>, Direction)> {
        &self.ghost_respawn_path
    }

    /// Get the position the ghosts go when eaten
    pub fn ghost_home_pos(&self) -> &(Point2<u8>, Direction) {
        &self.ghost_home_pos
    }
}

impl Default for PacmanAgentSetup {
    fn default() -> Self {
        let grid = ComputedGrid::try_from(crate::standard_grids::GRID_PACMAN);
        let pacman_start = (Point2::new(14, 7), Direction::Left);
        let ghosts = vec![
            GhostSetup {
                start_path: vec![
                    (Point2::new(13, 19), Direction::Left),
                    (Point2::new(12, 19), Direction::Left),
                ],
                color: GhostType::Red,
                scatter_point: Point2::new(26, 29),
            },
            GhostSetup {
                start_path: vec![
                    (Point2::new(14, 15), Direction::Up),
                    (Point2::new(14, 16), Direction::Up),
                    (Point2::new(14, 17), Direction::Up),
                    (Point2::new(14, 18), Direction::Up),
                    (Point2::new(14, 19), Direction::Up),
                ],
                color: GhostType::Pink,
                scatter_point: Point2::new(1, 29),
            },
            GhostSetup {
                start_path: vec![
                    (Point2::new(15, 15), Direction::Up),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(15, 16), Direction::Up),
                    (Point2::new(15, 17), Direction::Up),
                    (Point2::new(15, 16), Direction::Down),
                    (Point2::new(15, 15), Direction::Down),
                    (Point2::new(14, 15), Direction::Left),
                    (Point2::new(14, 16), Direction::Up),
                    (Point2::new(14, 17), Direction::Up),
                    (Point2::new(14, 18), Direction::Up),
                    (Point2::new(14, 19), Direction::Up),
                ],
                color: GhostType::Orange,
                scatter_point: Point2::new(1, 1),
            },
            GhostSetup {
                start_path: vec![
                    (Point2::new(12, 15), Direction::Up),
                    (Point2::new(12, 16), Direction::Up),
                    (Point2::new(12, 17), Direction::Up),
                    (Point2::new(12, 16), Direction::Down),
                    (Point2::new(12, 15), Direction::Down),
                    (Point2::new(12, 16), Direction::Up),
                    (Point2::new(12, 17), Direction::Up),
                    (Point2::new(12, 16), Direction::Down),
                    (Point2::new(12, 15), Direction::Down),
                    (Point2::new(12, 16), Direction::Up),
                    (Point2::new(12, 17), Direction::Up),
                    (Point2::new(12, 16), Direction::Down),
                    (Point2::new(12, 15), Direction::Down),
                    (Point2::new(12, 16), Direction::Up),
                    (Point2::new(12, 17), Direction::Up),
                    (Point2::new(12, 16), Direction::Down),
                    (Point2::new(12, 15), Direction::Down),
                    (Point2::new(12, 16), Direction::Up),
                    (Point2::new(12, 17), Direction::Up),
                    (Point2::new(12, 16), Direction::Down),
                    (Point2::new(12, 15), Direction::Down),
                    (Point2::new(13, 15), Direction::Right),
                    (Point2::new(13, 16), Direction::Up),
                    (Point2::new(13, 17), Direction::Up),
                    (Point2::new(13, 18), Direction::Up),
                    (Point2::new(13, 19), Direction::Up),
                ],
                color: GhostType::Blue,
                scatter_point: Point2::new(26, 1),
            },
        ];

        Self::new(
            grid.unwrap(),
            pacman_start,
            ghosts,
            vec![35, 135, 170, 270, 295, 395, 420],
            vec![
                (Point2::new(12, 15), Direction::Up),
                (Point2::new(12, 17), Direction::Up),
                (Point2::new(12, 16), Direction::Down),
                (Point2::new(12, 15), Direction::Down),
                (Point2::new(13, 15), Direction::Right),
                (Point2::new(13, 16), Direction::Up),
                (Point2::new(13, 17), Direction::Up),
                (Point2::new(13, 18), Direction::Up),
                (Point2::new(13, 19), Direction::Up),
            ],
        )
        .expect("Default PacmanAgentSetup is invalid")
    }
}

#[cfg(test)]
mod tests {
    use crate::agent_setup::PacmanAgentSetup;

    #[test]
    fn default_grid_setup() {
        PacmanAgentSetup::default();
    }
}
