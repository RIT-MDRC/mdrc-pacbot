use anyhow::{anyhow, Error};
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

fn validate_grid(grid: &Grid) -> Result<(), Error> {
    // the edges of the grid should all be walls
    for i in 0..GRID_HEIGHT {
        if grid[0][i] != GridValue::I {
            return Err(anyhow!("Left edge of grid is not all walls"));
        }
        if grid[GRID_HEIGHT - 1][i] != GridValue::I {
            return Err(anyhow!("Right edge of grid is not all walls"));
        }
    }
    for i in grid.iter().take(GRID_HEIGHT) {
        if i[0] != GridValue::I {
            return Err(anyhow!("Bottom edge of grid is not all walls"));
        }
        if i[GRID_WIDTH - 1] != GridValue::I {
            return Err(anyhow!("Top edge of grid is not all walls"));
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
    distance_matrix: Vec<Vec<Option<u8>>>,
}

impl TryFrom<Grid> for ComputedGrid {
    type Error = Error;

    fn try_from(grid: Grid) -> Result<Self, Self::Error> {
        validate_grid(&grid)?;

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
                let tile = grid[x][y];
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
                        grid[x + 1][y].walkable(),
                        grid[x - 1][y].walkable(),
                        grid[x][y + 1].walkable(),
                        grid[x][y - 1].walkable(),
                    ]);
                }
            }
        }

        for _ in 0..walkable_nodes.len() {
            distance_matrix.push(vec![None; walkable_nodes.len()]);
        }

        let mut s = ComputedGrid {
            grid,
            pellet_count,
            power_pellets,
            walkable_nodes,
            coords_to_node,
            valid_actions,
            distance_matrix,
        };

        // compute distance matrix
        for (i, &start) in s.walkable_nodes.iter().enumerate() {
            let mut visited = vec![false; s.walkable_nodes.len()];
            let mut queue = vec![(start, 0)];
            while let Some((pos, dist)) = queue.pop() {
                let node_index = *s.coords_to_node.get(&pos).unwrap();
                if visited[node_index] {
                    continue;
                }
                visited[node_index] = true;
                s.distance_matrix[i][node_index] = Some(dist);
                for &neighbor in &[
                    Point2::new(pos.x + 1, pos.y),
                    Point2::new(pos.x - 1, pos.y),
                    Point2::new(pos.x, pos.y + 1),
                    Point2::new(pos.x, pos.y - 1),
                ] {
                    if s.coords_to_node.get(&neighbor).is_some() {
                        queue.push((neighbor, dist + 1));
                    }
                }
            }
        }

        Ok(s)
    }
}

impl ComputedGrid {
    pub fn at(&self, p: &Point2<u8>) -> Option<GridValue> {
        if p.x >= GRID_WIDTH as u8 || p.y >= GRID_HEIGHT as u8 {
            return None;
        }
        Some(self.grid[p.x as usize][p.y as usize])
    }

    pub fn next(&self, p: &Point2<u8>, direction: &Direction) -> Option<GridValue> {
        let p = match direction {
            Direction::Right => Point2::new(p.x + 1, p.y),
            Direction::Left => {
                if p.x == 0 {
                    return None;
                }
                Point2::new(p.x - 1, p.y)
            }
            Direction::Up => Point2::new(p.x, p.y + 1),
            Direction::Down => {
                if p.y == 0 {
                    return None;
                }
                Point2::new(p.x, p.y - 1)
            }
        };
        self.at(&p)
    }

    pub fn dist(&self, p1: &Point2<u8>, p2: &Point2<u8>) -> Option<u8> {
        let p1 = self.coords_to_node.get(p1)?;
        let p2 = self.coords_to_node.get(p2)?;
        self.distance_matrix[*p1][*p2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridValue::{e as EMPTY, o as PELLET, I as WALL, O as POWER_PELLET};
    use crate::standard_grids::*;

    #[test]
    fn valid_preset_grids() {
        assert!(validate_grid(&GRID_PACMAN).is_ok());
        assert!(validate_grid(&GRID_BLANK).is_ok());
    }

    #[test]
    fn validation_require_empty_space() {
        let mut grid = GRID_BLANK;
        grid[1][1] = WALL;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(format!("{}", v.unwrap_err()), "No walkable spaces");
    }

    #[test]
    fn validation_invalid_bottom_wall() {
        let mut grid = GRID_BLANK;
        grid[1][0] = EMPTY;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "Bottom edge of grid is not all walls"
        );
    }

    #[test]
    fn validation_invalid_top_wall() {
        let mut grid = GRID_BLANK;
        grid[1][GRID_HEIGHT - 1] = EMPTY;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "Top edge of grid is not all walls"
        );
    }

    #[test]
    fn validation_invalid_left_wall() {
        let mut grid = GRID_BLANK;
        grid[0][1] = EMPTY;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "Left edge of grid is not all walls"
        );
    }

    #[test]
    fn validation_invalid_right_wall() {
        let mut grid = GRID_BLANK;
        grid[GRID_WIDTH - 1][1] = EMPTY;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "Right edge of grid is not all walls"
        );
    }

    #[test]
    fn validation_invalid_2x2() {
        let mut grid = GRID_BLANK;
        grid[1][1] = EMPTY;
        grid[1][2] = EMPTY;
        grid[2][1] = EMPTY;
        grid[2][2] = EMPTY;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "2x2 walkable square at (1, 1)"
        );
    }

    #[test]
    fn compute_preset_grids() {
        ComputedGrid::try_from(GRID_PACMAN).unwrap();
        ComputedGrid::try_from(GRID_BLANK).unwrap();
    }

    #[test]
    fn compute_pellet_count() {
        let mut grid = GRID_BLANK;
        grid[1][1] = PELLET;
        grid[1][2] = PELLET;
        grid[6][1] = PELLET;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.pellet_count, 3);
    }

    #[test]
    fn compute_power_pellets() {
        let mut grid = GRID_BLANK;
        grid[1][1] = POWER_PELLET;
        grid[1][2] = POWER_PELLET;
        grid[6][1] = POWER_PELLET;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.power_pellets.len(), 3);
        assert!(computed_grid.power_pellets.contains(&Point2::new(1, 1)));
        assert!(computed_grid.power_pellets.contains(&Point2::new(1, 2)));
        assert!(computed_grid.power_pellets.contains(&Point2::new(6, 1)));
    }

    #[test]
    fn compute_walkable_nodes() {
        let mut grid = GRID_BLANK;
        grid[1][1] = PELLET;
        grid[1][2] = PELLET;
        grid[6][1] = PELLET;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.walkable_nodes.len(), 3);
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(1, 1)));
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(1, 2)));
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(6, 1)));
    }

    #[test]
    fn compute_coords_to_node() {
        let mut grid = GRID_BLANK;
        grid[1][1] = PELLET;
        grid[1][2] = PELLET;
        grid[6][1] = PELLET;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.coords_to_node.len(), 3);
        let idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(1, 1))
            .unwrap();
        assert_eq!(computed_grid.walkable_nodes[idx], Point2::new(1, 1));
    }

    #[test]
    fn compute_valid_actions() {
        let mut grid = GRID_BLANK;
        grid[1][1] = PELLET;
        grid[1][2] = PELLET;
        grid[6][1] = PELLET;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.valid_actions.len(), 3);
        let one_one_idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(1, 1))
            .unwrap();
        assert_eq!(
            computed_grid.valid_actions[one_one_idx],
            [true, false, false, true, false]
        );

        let one_two_idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(1, 2))
            .unwrap();
        assert_eq!(
            computed_grid.valid_actions[one_two_idx],
            [true, false, false, false, true]
        );

        let six_one_idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(6, 1))
            .unwrap();
        assert_eq!(
            computed_grid.valid_actions[six_one_idx],
            [true, false, false, false, false]
        );
    }

    #[test]
    fn compute_distance_matrix() {
        let mut grid = GRID_BLANK;
        grid[1][1] = PELLET;
        grid[1][2] = PELLET;
        grid[6][1] = PELLET;

        let points = vec![Point2::new(1, 1), Point2::new(1, 2), Point2::new(6, 1)];

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.distance_matrix.len(), 3);
        assert_eq!(computed_grid.distance_matrix[0].len(), 3);
        assert_eq!(computed_grid.distance_matrix[1].len(), 3);
        assert_eq!(computed_grid.distance_matrix[2].len(), 3);
        assert_eq!(computed_grid.dist(&points[0], &points[0]), Some(0));
        assert_eq!(computed_grid.dist(&points[0], &points[1]), Some(1));
        assert_eq!(computed_grid.dist(&points[0], &points[2]), None);
        assert_eq!(computed_grid.dist(&points[1], &points[0]), Some(1));
        assert_eq!(computed_grid.dist(&points[1], &points[1]), Some(0));
        assert_eq!(computed_grid.dist(&points[1], &points[2]), None);
        assert_eq!(computed_grid.dist(&points[2], &points[0]), None);
        assert_eq!(computed_grid.dist(&points[2], &points[1]), None);
        assert_eq!(computed_grid.dist(&points[2], &points[2]), Some(0));
    }

    #[test]
    fn grid_next() {
        let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
        assert_eq!(grid.next(&Point2::new(0, 0), &Direction::Right), Some(WALL));
        assert_eq!(
            grid.next(&Point2::new(0, 1), &Direction::Right),
            Some(EMPTY)
        );
        assert_eq!(grid.next(&Point2::new(1, 0), &Direction::Up), Some(EMPTY));
    }

    #[test]
    fn grid_next_oob() {
        let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
        assert_eq!(grid.next(&Point2::new(0, 0), &Direction::Left), None);
        assert_eq!(grid.next(&Point2::new(0, 0), &Direction::Down), None);
        assert_eq!(
            grid.next(&Point2::new(0, (GRID_HEIGHT - 1) as u8), &Direction::Up),
            None
        );
        assert_eq!(
            grid.next(&Point2::new((GRID_WIDTH - 1) as u8, 0), &Direction::Right),
            None
        );
    }

    #[test]
    fn grid_at() {
        let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
        assert_eq!(grid.at(&Point2::new(0, 0)), Some(WALL));
    }

    #[test]
    fn grid_at_oob() {
        let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
        assert_eq!(grid.at(&Point2::new(0, GRID_HEIGHT as u8)), None);
        assert_eq!(grid.at(&Point2::new(GRID_WIDTH as u8, 0)), None);
    }
}
