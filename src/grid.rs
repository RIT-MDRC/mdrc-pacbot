use std::iter::Map;
use anyhow::Error;
use rapier2d::na::Vector2;

pub const GRID_WIDTH: usize = 32;
pub const GRID_HEIGHT: usize = 32;

pub type Grid = [[u8; GRID_WIDTH]; GRID_HEIGHT];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComputedGrid {
    grid: Grid,

    pellet_count: u32,
    power_pellets: Vec<Vector2<u8>>,

    walkable_nodes: Vec<Vector2<u8>>,
    coords_to_node: Map<Vector2<u8>, usize>,

    valid_actions: Vec<[bool; 5]>,
    distance_matrix: Vec<Vec<u8>>,
}

impl TryFrom<Grid> for ComputedGrid {
    type Error = Error;

    fn try_from(grid: Grid) -> Result<Self, Self::Error> {
        todo!()
    }
}