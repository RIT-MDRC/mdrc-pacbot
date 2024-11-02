//! [`Grid`] is the integer-based 2D array that gives the locations of the walls

#[cfg(feature = "std")]
pub mod computed_grid;
pub mod standard_grid;

/// Width and height of a [`Grid`].
pub const GRID_SIZE: usize = 32;

/// A 2D grid of walls
///
/// The grid is indexed by `grid[row][col]` or (in most contexts) `grid[x][y]`
pub type Grid = [[bool; GRID_SIZE]; GRID_SIZE];
