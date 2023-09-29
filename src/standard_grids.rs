#![cfg_attr(rustfmt, rustfmt_skip)]
//! A set of pre-made general purpose grids

use crate::grid::Grid;
use crate::grid::GridValue::*;

/// The official Pacbot [`Grid`]
/// 
/// Out-of-bounds areas are replaced with walls to adhere to ComputedGrid rules
/// 
/// ```
/// use mdrc_pacbot_util::standard_grids::GRID_PACMAN;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_PACMAN;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_PACMAN: Grid = [
//  bottom left of pacman board                                           // top left of pacman board
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I], // 0
    [I, o, o, o, o, I, I, O, o, o, o, I, I, I, I, I, I, I, I, I, I, I, o, o, o, o, o, O, o, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I], // 5
    [I, o, I, I, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, e, e, e, e, e, e, e, e, e, I, I, o, o, o, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, I, I, I, I, e, I, I, o, I, I, o, I, I, I, o, I, I], // 10
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, n, n, n, I, e, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, o, o, o, I, I, o, o, o, o, I, I, e, I, n, n, n, I, e, e, e, o, I, I, o, o, o, o, o, I, I],
    [I, o, I, I, I, I, I, e, I, I, I, I, I, e, I, n, n, n, n, e, I, I, I, I, I, o, I, I, I, I, I, I],
    [I, o, I, I, I, I, I, e, I, I, I, I, I, e, I, n, n, n, n, e, I, I, I, I, I, o, I, I, I, I, I, I],
    [I, o, o, o, o, I, I, o, o, o, o, I, I, e, I, n, n, n, I, e, e, e, o, I, I, o, o, o, o, o, I, I], // 15
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, n, n, n, I, e, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, I, I, I, I, e, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, e, e, e, e, e, e, e, e, e, I, I, o, o, o, o, I, I, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I], // 20
    [I, o, I, I, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I], // 25
    [I, o, o, o, o, I, I, O, o, o, o, I, I, I, I, I, I, I, I, I, I, I, o, o, o, o, o, O, o, o, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
//   |              |              |              |              |              |              |   top right of pacman board
//   0              5              10             15             20             25             30
];

/// A (mostly) blank [`Grid`] - (1, 1) is walkable
///
/// ```
/// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_BLANK;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_BLANK: Grid = [
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I]
];

/// A [`Grid`] where the outermost path is empty
///
/// ```
/// use mdrc_pacbot_util::standard_grids::GRID_OUTER;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_OUTER;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_OUTER: Grid = [
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I]
];

/// A [`Grid`] with many smaller paths to practice maneuvering
///
/// ```
/// use mdrc_pacbot_util::standard_grids::GRID_PLAYGROUND;
/// use mdrc_pacbot_util::grid::{ComputedGrid, Grid};
///
/// let grid: Grid = GRID_PLAYGROUND;
/// let computed_grid: ComputedGrid = grid.try_into().unwrap();
/// ```
pub const GRID_PLAYGROUND: Grid = [
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, I],
    [I, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I],
    [I, e, I, I, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, e, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, e, e, e, I, I, I, I, I, e, e, e, e, I, I, e, e, e, e, I, I, e, e, e, e, I, I, I],
    [I, e, I, I, e, I, I, e, I, I, I, I, I, e, I, I, e, e, e, e, I, I, e, e, e, e, I, I, e, I, I, I],
    [I, e, I, I, e, I, I, e, I, I, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I],
    [I, e, I, I, e, e, e, e, I, I, I, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I, I, I, I, I, e, e, e, e, e, I, I, e, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, e, e, e, I, I, e, e, e, e, I, I, I, e, e, e, e, I, I, I],
    [I, e, I, I, e, e, e, e, e, e, I, I, I, I, I, e, e, e, e, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, I, I, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, I, I, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, e, I, I, e, e, I, I, e, e, I, I, e, e, e, e, I, I, e, e, e, e, I, I, e, e, e, e, I, I, I, I],
    [I, e, I, I, I, e, I, I, e, I, I, I, e, I, I, e, I, I, e, I, I, e, I, I, e, I, I, e, I, I, I, I],
    [I, e, I, I, I, e, I, I, e, I, I, I, e, I, I, e, e, e, e, I, I, e, e, e, e, I, I, e, I, I, I, I],
    [I, e, I, I, e, e, I, I, e, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I, I],
    [I, e, I, I, e, I, I, I, I, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, e, e, I, I],
    [I, e, I, I, e, I, I, I, I, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I],
    [I, e, I, I, e, e, e, e, e, e, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, e, e, e, I, I, I, I, I, I, I, I, I, I, I, I, e, e, e, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, e, e, e, I, I, e, e, e, e, I, I, e, e, e, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, I, I, e, I, I, e, I, I, e, I, I, I, I, I, I],
    [I, e, I, I, I, I, I, I, I, I, I, I, I, I, I, I, e, e, e, e, I, I, e, e, e, e, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I]
];