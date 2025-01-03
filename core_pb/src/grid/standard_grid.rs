#[cfg(feature = "std")]
use crate::grid::computed_grid::ComputedGrid;
use crate::grid::Grid;
#[cfg(feature = "std")]
use core::f32::consts::PI;
use nalgebra::Point2;
#[cfg(feature = "std")]
use nalgebra::{Isometry2, Vector2};
#[cfg(feature = "std")]
use pacbot_rs::variables::PACMAN_SPAWN_LOC;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize)]
pub enum StandardGrid {
    #[default]
    Pacman,
    Playground,
    Blank,
    Outer,
    Open,
}

#[allow(dead_code)]
impl StandardGrid {
    /// Get a list of all available grids
    pub fn get_all() -> [Self; 5] {
        [
            Self::Pacman,
            Self::Playground,
            Self::Outer,
            Self::Blank,
            Self::Open,
        ]
    }

    /// Get the [`Grid`] associated with this enum
    ///
    /// From generated file
    pub fn get_grid(&self) -> Grid {
        match self {
            Self::Pacman => GRID_PACMAN,
            Self::Playground => GRID_PLAYGROUND,
            Self::Outer => GRID_OUTER,
            Self::Blank => GRID_BLANK,
            Self::Open => GRID_OPEN,
        }
    }

    /// Get the [`ComputedGrid`] associated with this enum
    #[cfg(feature = "std")]
    pub fn compute_grid(self) -> ComputedGrid {
        self.into()
    }

    /// Get the default Pacbot [`Isometry2`] associated with this enum
    #[cfg(feature = "std")]
    pub fn get_default_pacbot_isometry(&self) -> Isometry2<f32> {
        match self {
            StandardGrid::Pacman => Isometry2::new(
                Vector2::new(PACMAN_SPAWN_LOC.row as f32, PACMAN_SPAWN_LOC.col as f32),
                PI / 2.0,
            ),
            StandardGrid::Playground => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
            StandardGrid::Outer => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
            StandardGrid::Blank => Isometry2::new(Vector2::new(1.0, 1.0), 0.0),
            StandardGrid::Open => Isometry2::new(Vector2::new(16.0, 16.0), 0.0),
        }
    }

    /// Get the part of the [`Grid`] that should actually show on the gui
    pub fn get_soft_boundaries(&self) -> (Point2<f32>, Point2<f32>) {
        match self {
            Self::Pacman => (Point2::new(-1.0, -1.0), Point2::new(31.0, 28.0)),
            Self::Playground => (Point2::new(25.0, 22.0), Point2::new(32.0, 32.0)),
            _ => (Point2::new(-1.0, -1.0), Point2::new(32.0, 32.0)),
        }
    }

    /// Get the rectangles (in grid coordinates) that should be repainted with the background color
    #[cfg(feature = "std")]
    pub fn get_outside_soft_boundaries(&self) -> Vec<(Point2<f32>, Point2<f32>)> {
        match self {
            Self::Pacman => vec![
                (Point2::new(-1.0, 28.0), Point2::new(32.1, 32.1)),
                (Point2::new(31.0, -1.0), Point2::new(32.1, 32.1)),
            ],
            Self::Playground => vec![
                (Point2::new(-1.0, -1.0), Point2::new(25.0, 32.1)),
                (Point2::new(-1.0, -1.0), Point2::new(32.1, 22.0)),
            ],
            _ => vec![],
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/generated_grids.rs"));

/// The official Pacbot [`Grid`]
///
/// Out-of-bounds areas are replaced with walls to adhere to ComputedGrid rules
///
/// ```
/// use core_pb::grid::standard_grid::GRID_PACMAN;
/// use core_pb::grid::Grid;
/// use core_pb::grid::computed_grid::ComputedGrid;
///
/// let grid: Grid = GRID_PACMAN;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
#[rustfmt::skip]
pub const GRID_PACMAN: Grid = GENERATED_GRID_PACMAN;

/// A (mostly) blank [`Grid`] - (1, 1) is walkable
///
/// ```
/// use core_pb::grid::standard_grid::GRID_BLANK;
/// use core_pb::grid::Grid;
/// use core_pb::grid::computed_grid::ComputedGrid;
///
/// let grid: Grid = GRID_BLANK;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
#[rustfmt::skip]
pub const GRID_BLANK: Grid = GENERATED_GRID_BLANK;

/// A special [`Grid`] with no internal walls
///
/// ```
/// use core_pb::grid::standard_grid::GRID_OPEN;
/// use core_pb::grid::Grid;
/// use core_pb::grid::computed_grid::ComputedGrid;
///
/// let grid: Grid = GRID_OPEN;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
#[rustfmt::skip]
pub const GRID_OPEN: Grid = GENERATED_GRID_OPEN;

/// A [`Grid`] where the outermost path is empty
///
/// ```
/// use core_pb::grid::standard_grid::GRID_OUTER;
/// use core_pb::grid::Grid;
/// use core_pb::grid::computed_grid::ComputedGrid;
///
/// let grid: Grid = GRID_OUTER;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
#[rustfmt::skip]
pub const GRID_OUTER: Grid = GENERATED_GRID_OUTER;

/// A [`Grid`] with many smaller paths to practice maneuvering
///
/// ```
/// use core_pb::grid::standard_grid::GRID_PLAYGROUND;
/// use core_pb::grid::Grid;
/// use core_pb::grid::computed_grid::ComputedGrid;
///
/// let grid: Grid = GRID_PLAYGROUND;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
#[rustfmt::skip]
pub const GRID_PLAYGROUND: Grid = GENERATED_GRID_PLAYGROUND;
