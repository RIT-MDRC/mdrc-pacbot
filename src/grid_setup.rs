//! Static information needed to set up a Pacman game
use crate::game_state::GhostType;
use crate::grid::ComputedGrid;
use crate::grid::GridValue::{o, O};
use anyhow::{anyhow, Error};
use rapier2d::na::Point2;

/// Static information needed to set up a ghost for Pacman game
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GhostSetup {
    /// The ghost's starting path; where it goes when it first spawns
    pub start_path: Vec<Point2<u8>>,
    /// The ghost's color; determines behavior
    pub color: GhostType,
    /// The ghost's scatter point; where it goes when it's not chasing Pacman
    pub scatter_point: Point2<u8>,
}

/// Static information needed to set up a Pacman game
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PacmanGridSetup {
    /// The grid
    grid: ComputedGrid,
    /// The Pacman start point
    pacman_start: Point2<u8>,
    /// The ghosts
    ghosts: Vec<GhostSetup>,
}

impl PacmanGridSetup {
    /// Create a new PacmanGridSetup from a grid and a list of ghost start points
    pub fn new(
        grid: ComputedGrid,
        pacman_start: Point2<u8>,
        ghosts: Vec<GhostSetup>,
    ) -> Result<Self, Error> {
        let start_value = grid
            .at(&pacman_start)
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
            if !grid
                .at(&ghost.scatter_point)
                .ok_or(anyhow!("Ghost path position doesn't exist"))?
                .walkable()
            {
                return Err(anyhow!("Ghost scatter point is not walkable"));
            }
            for point in &ghost.start_path {
                if !grid
                    .at(point)
                    .ok_or(anyhow!("Ghost path position doesn't exist"))?
                    .walkable()
                {
                    return Err(anyhow!("Ghost start path is not walkable"));
                }
            }
        }

        Ok(Self {
            grid,
            pacman_start,
            ghosts,
        })
    }

    /// Get the grid
    pub fn grid(&self) -> &ComputedGrid {
        &self.grid
    }

    /// Get the Pacman start point
    pub fn pacman_start(&self) -> &Point2<u8> {
        &self.pacman_start
    }

    /// Get the ghosts
    pub fn ghosts(&self) -> &Vec<GhostSetup> {
        &self.ghosts
    }

    pub fn default() -> Self {
        let grid = ComputedGrid::try_from(crate::standard_grids::GRID_PACMAN);
        let pacman_start = Point2::new(14, 7);
        let ghosts = vec![
            GhostSetup {
                start_path: vec![], // TODO
                color: GhostType::Red,
                scatter_point: Point2::new(0, 0), // TODO
            },
            GhostSetup {
                start_path: vec![], // TODO
                color: GhostType::Pink,
                scatter_point: Point2::new(0, 0), // TODO
            },
            GhostSetup {
                start_path: vec![], // TODO
                color: GhostType::Orange,
                scatter_point: Point2::new(0, 0), // TODO
            },
            GhostSetup {
                start_path: vec![], // TODO
                color: GhostType::Blue,
                scatter_point: Point2::new(0, 0), // TODO
            },
        ];

        Self {
            grid: grid.unwrap(),
            pacman_start,
            ghosts,
        }
    }
}
