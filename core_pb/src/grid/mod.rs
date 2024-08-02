#[cfg(feature = "std")]
pub mod computed_grid;
pub mod standard_grid;

/// Width and height of a [`Grid`].
pub const GRID_SIZE: usize = 32;

/// A 2D grid
///
/// The grid is indexed by `grid[row][col]`
pub type Grid = [[bool; GRID_SIZE]; GRID_SIZE];
