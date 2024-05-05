#![cfg_attr(rustfmt, rustfmt_skip)]
//! A set of pre-made general purpose grids

use std::f32::consts::PI;
use nalgebra::{Isometry2, Point2, Vector2};
use pacbot_rs::variables::PACMAN_SPAWN_LOC;
use serde::{Deserialize, Serialize};
use crate::grid::{ComputedGrid, Grid};

/// An enum to support egui grid selection
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardGrid {
    /// The official Pacbot [`Grid`]
    Pacman,
    /// A [`Grid`] with many smaller paths to practice maneuvering
    Playground,
    /// A [`Grid`] where the outermost path is empty
    Outer,
    /// A (mostly) blank [`Grid`] - (1, 1) is walkable
    Blank,
}

impl Default for StandardGrid {
    fn default() -> Self {
        Self::Pacman
    }
}

impl StandardGrid {
    /// Get a list of all available grids
    pub fn get_all() -> Vec<Self> {
        vec![Self::Pacman, Self::Playground, Self::Outer, Self::Blank]
    }

    /// Get the [`Grid`] associated with this enum
    pub fn get_grid(&self) -> Grid {
        match self {
            Self::Pacman => GRID_PACMAN,
            Self::Playground => GRID_PLAYGROUND,
            Self::Outer => GRID_OUTER,
            Self::Blank => GRID_BLANK,
        }
    }

    /// Get the [`ComputedGrid`] associated with this enum
    pub fn compute_grid(&self) -> ComputedGrid {
        ComputedGrid::try_from(self.get_grid()).expect("Failed to compute a StandardGrid")
    }

    /// Get the default Pacbot [`Isometry2`] associated with this enum
    pub fn get_default_pacbot_isometry(&self) -> Isometry2<f32> {
        match self {
            StandardGrid::Pacman => Isometry2::new(Vector2::new(PACMAN_SPAWN_LOC.row as f32, PACMAN_SPAWN_LOC.col as f32), PI / 2.0),
            StandardGrid::Playground => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
            StandardGrid::Outer => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
            StandardGrid::Blank => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
        }
    }

    /// Get the part of the [`Grid`] that should actually show on the gui
    pub fn get_soft_boundaries(&self) -> (Point2<f32>, Point2<f32>) {
        match self {
            Self::Pacman => (Point2::new(-1.0, -1.0), Point2::new(31.0, 28.0)),
            _ => (Point2::new(-1.0, -1.0), Point2::new(32.0, 32.0))
        }
    }

    /// Get the rectangles (in grid coordinates) that should be repainted with the background color
    pub fn get_outside_soft_boundaries(&self) -> Vec<(Point2<f32>, Point2<f32>)> {
        match self {
            Self::Pacman => vec![
                (Point2::new(-1.0, 28.0), Point2::new(32.1, 32.1)),
                (Point2::new(31.0, -1.0), Point2::new(32.1, 32.1)),
            ],
            _ => vec![]
        }
    }
}

const W: bool = true;
const O: bool = false;

/// The official Pacbot [`Grid`]
///
/// Out-of-bounds areas are replaced with walls to adhere to ComputedGrid rules
///
/// ```
/// use mdrc_pacbot_util::grid::standard_grids::GRID_PACMAN;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_PACMAN;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_PACMAN: Grid = [
//  bottom left of pacman board                                           // top left of pacman board
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W], // 0
    [W, O, O, O, O, O, O, O, O, O, O, O, O, W, W, O, O, O, O, O, O, O, O, O, O, O, O, W, W, W, W, W],
    [W, O, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, W, W, W, W, W], // 5
    [W, O, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, O, O, O, O, O, W, W, O, O, O, O, W, W, O, O, O, O, W, W, O, O, O, O, O, O, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W], // 10
    [W, W, W, W, W, W, O, W, W, O, O, O, O, O, O, O, O, O, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, O, O, O, W, W, W, W, W, W, W, W, O, O, O, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W], // 15
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, O, O, O, O, O, O, O, O, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W],
    [W, O, O, O, O, O, O, O, O, O, O, O, O, W, W, O, O, O, O, O, O, O, O, O, O, O, O, W, W, W, W, W], // 20
    [W, O, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W, W, W, W, O, W, W, W, W, W],
    [W, O, O, O, W, W, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, W, W, O, O, O, W, W, W, W, W],
    [W, W, W, O, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, O, W, W, W, W, W, W, W],
    [W, W, W, O, W, W, O, W, W, O, W, W, W, W, W, W, W, W, O, W, W, O, W, W, O, W, W, W, W, W, W, W], // 25
    [W, O, O, O, O, O, O, W, W, O, O, O, O, W, W, O, O, O, O, W, W, O, O, O, O, O, O, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W],
    [W, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
//   |              |              |              |              |              |              |   top right of pacman board
//   0              5              10             15             20             25             30
];

/// A (mostly) blank [`Grid`] - (1, 1) is walkable
///
/// ```
/// use mdrc_pacbot_util::grid::standard_grids::GRID_BLANK;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_BLANK;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_BLANK: Grid = [
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W]
];

/// A [`Grid`] where the outermost path is empty
///
/// ```
/// use mdrc_pacbot_util::grid::standard_grids::GRID_OUTER;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_OUTER;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_OUTER: Grid = [
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W],
    [W, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W]
];

/// A [`Grid`] with many smaller paths to practice maneuvering
///
/// ```
/// use mdrc_pacbot_util::grid::standard_grids::GRID_PLAYGROUND;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_PLAYGROUND;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_PLAYGROUND: Grid = [
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, O, O, O, O, O, O, O, O, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, O, O, O, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, O, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, O, O, O, W, W, W, W, W, W, W],
    [W, O, W, W, W, O, O, O, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, W, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, O, W, W, O, W],
    [W, O, W, W, W, O, O, O, O, O, O, O, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W],
    [W, O, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, W, W, W, W, W, W, O, O, O, O, O, O, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, O, W, W, O, W, W, O, W],
    [W, O, W, W, W, W, W, W, W, W, W, W, W, W, O, W, W, W, W, W, W, W, W, W, O, O, O, O, O, O, O, W],
    [W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W, W]
];