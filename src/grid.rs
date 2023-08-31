use anyhow::Error;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rapier2d::na::Point2;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Direction {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

/// Enum for grid cell values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum GridValue {
    /// Wall
    I = 1,
    /// Normal pellet
    o = 2,
    /// Empty space
    e = 3,
    /// Power pellet
    O = 4,
    /// Ghost chambers
    n = 5,
    /// Cherry position
    c = 6,
}

impl GridValue {
    pub fn walkable(self) -> bool {
        self != GridValue::I && self != GridValue::n
    }
}

pub const GRID_WIDTH: usize = 32;
pub const GRID_HEIGHT: usize = 32;

pub type Grid = [[GridValue; GRID_WIDTH]; GRID_HEIGHT];

fn validate_grid(grid: Grid) -> Result<(), Error> {
    // the edges of the grid should all be walls
    for i in 0..GRID_WIDTH {
        if grid[0][i] != GridValue::I {
            return Err(Error::msg("Top edge of grid is not all walls"));
        }
        if grid[GRID_HEIGHT - 1][i] != GridValue::I {
            return Err(Error::msg("Bottom edge of grid is not all walls"));
        }
    }
    for i in grid.iter().take(GRID_HEIGHT) {
        if i[0] != GridValue::I {
            return Err(Error::msg("Left edge of grid is not all walls"));
        }
        if i[GRID_WIDTH - 1] != GridValue::I {
            return Err(Error::msg("Right edge of grid is not all walls"));
        }
    }

    // there should be no 2x2 walkable squares
    for i in 0..GRID_HEIGHT - 1 {
        for j in 0..GRID_WIDTH - 1 {
            if grid[i][j].walkable()
                && grid[i][j + 1].walkable()
                && grid[i + 1][j].walkable()
                && grid[i + 1][j + 1].walkable()
            {
                return Err(Error::msg(format!("2x2 walkable square at ({}, {})", i, j)));
            }
        }
    }

    // there should be at least one walkable space
    let mut walkable = false;
    for row in grid.iter().take(GRID_HEIGHT) {
        for cell in row.iter().take(GRID_WIDTH) {
            if cell.walkable() {
                walkable = true;
                break;
            }
        }
    }
    if !walkable {
        return Err(Error::msg("No walkable spaces"));
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedGrid {
    grid: Grid,

    pellet_count: u32,
    power_pellets: Vec<Point2<u8>>,

    walkable_nodes: Vec<Point2<u8>>,
    coords_to_node: HashMap<Point2<u8>, usize>,

    valid_actions: Vec<[bool; 5]>,
    distance_matrix: Vec<Vec<u8>>,
}

impl TryFrom<Grid> for ComputedGrid {
    type Error = Error;

    fn try_from(grid: Grid) -> Result<Self, Self::Error> {
        validate_grid(grid)?;

        let mut pellet_count = 0;
        let mut power_pellets = vec![];

        let mut walkable_nodes = vec![];
        let mut coords_to_node: HashMap<Point2<u8>, usize> = HashMap::new();

        let mut valid_actions = vec![];
        let mut distance_matrix = vec![];

        // note that all edges must be walls
        for y in 1..GRID_HEIGHT - 1 {
            for x in 1..GRID_WIDTH - 1 {
                let pos = Point2::new(x as u8, y as u8);
                let tile = grid[y][x];
                if tile == GridValue::o {
                    pellet_count += 1;
                } else if tile == GridValue::O {
                    power_pellets.push(pos);
                }
                if tile.walkable() {
                    let node_index = walkable_nodes.len();
                    walkable_nodes.push(pos);
                    coords_to_node.insert(pos, node_index);
                    valid_actions.push([
                        true,
                        grid[y][x + 1].walkable(),
                        grid[y][x - 1].walkable(),
                        grid[y - 1][x].walkable(),
                        grid[y + 1][x].walkable(),
                    ]);
                    distance_matrix.push(vec![0; walkable_nodes.len()]);
                }
            }
        }

        // compute distance matrix
        for (i, &start) in walkable_nodes.iter().enumerate() {
            let mut visited = vec![false; walkable_nodes.len()];
            let mut queue = vec![(start, 0)];
            while let Some((pos, dist)) = queue.pop() {
                let node_index = coords_to_node[&pos];
                if visited[node_index] {
                    continue;
                }
                visited[node_index] = true;
                distance_matrix[i][node_index] = dist;
                for &neighbor in &[
                    Point2::new(pos.x + 1, pos.y),
                    Point2::new(pos.x - 1, pos.y),
                    Point2::new(pos.x, pos.y + 1),
                    Point2::new(pos.x, pos.y - 1),
                ] {
                    if coords_to_node.get(&neighbor).is_some() {
                        queue.push((neighbor, dist + 1));
                    }
                }
            }
        }

        Ok(ComputedGrid {
            grid,
            pellet_count,
            power_pellets,
            walkable_nodes,
            coords_to_node,
            valid_actions,
            distance_matrix,
        })
    }
}

impl ComputedGrid {
    pub fn at(&self, p: Point2<u8>) -> Option<GridValue> {
        if p.x < 0 || p.x >= GRID_WIDTH as u8 || p.y < 0 || p.y >= GRID_HEIGHT as u8 {
            return None;
        }
        Some(self.grid[p.x][p.y])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_preset_grids() {
        assert!(validate_grid(crate::alternate_grids::GRID_PACMAN).is_ok());
        assert!(validate_grid(crate::alternate_grids::GRID_BLANK).is_ok());
    }
}
