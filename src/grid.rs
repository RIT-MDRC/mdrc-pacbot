//! Logical grid structs and utilities.

use anyhow::{anyhow, Error};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rapier2d::na::Point2;
use std::collections::HashMap;

/// Enum for direction values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Direction {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

/// Enum for [`Grid`] cell values.
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
    /// Returns whether this [`GridValue`] is walkable.
    ///
    /// A [`GridValue`] is walkable if it is not a ghost chamber or wall.
    pub fn walkable(self) -> bool {
        self != GridValue::I && self != GridValue::n
    }
}

/// Width of a [`Grid`].
pub const GRID_WIDTH: usize = 32;
/// Height of a [`Grid`].
pub const GRID_HEIGHT: usize = 32;

/// A 2D grid of [`GridValue`]s.
///
/// The grid is indexed by `grid[x][y]`, where `x` is visually horizontal and `y` is vertical.
pub type Grid = [[GridValue; GRID_WIDTH]; GRID_HEIGHT];

/// Validates a [`Grid`].
///
/// A valid [`Grid`] must satisfy the following conditions:
/// - The edges of the grid must all be walls.
/// - There must be no 2x2 walkable squares.
/// - There must be at least one walkable space.
/// - No wall should have a walkable cell either both above and below or both to the left and right
fn validate_grid(grid: &Grid) -> Result<(), Error> {
    // the edges of the grid should all be walls
    if (0..GRID_HEIGHT).any(|y| grid[0][y] != GridValue::I) {
        return Err(anyhow!("Left edge of grid is not all walls"));
    }
    if (0..GRID_HEIGHT).any(|y| grid[GRID_WIDTH - 1][y] != GridValue::I) {
        return Err(anyhow!("Right edge of grid is not all walls"));
    }
    if (0..GRID_WIDTH).any(|x| grid[x][0] != GridValue::I) {
        return Err(anyhow!("Bottom edge of grid is not all walls"));
    }
    if (0..GRID_WIDTH).any(|x| grid[x][GRID_HEIGHT - 1] != GridValue::I) {
        return Err(anyhow!("Top edge of grid is not all walls"));
    }

    // there should be no 2x2 walkable squares
    for x in 0..GRID_HEIGHT - 1 {
        for y in 0..GRID_WIDTH - 1 {
            if grid[x][y].walkable()
                && grid[x][y + 1].walkable()
                && grid[x + 1][y].walkable()
                && grid[x + 1][y + 1].walkable()
            {
                return Err(Error::msg(format!("2x2 walkable square at ({}, {})", x, y)));
            }
        }
    }

    // there should be at least one walkable space
    if !grid
        .iter()
        .any(|row| row.iter().any(|cell| cell.walkable()))
    {
        return Err(Error::msg("No walkable spaces"));
    }

    // no wall should have a walkable cell either both above and below or both to the left and right
    for x in 1..GRID_HEIGHT - 1 {
        for y in 1..GRID_WIDTH - 1 {
            if grid[x][y] == GridValue::I {
                if grid[x - 1][y].walkable() && grid[x + 1][y].walkable() {
                    return Err(Error::msg(format!(
                        "Wall at ({}, {}) has walkable cells both above and below",
                        x, y
                    )));
                }
                if grid[x][y - 1].walkable() && grid[x][y + 1].walkable() {
                    return Err(Error::msg(format!(
                        "Wall at ({}, {}) has walkable cells both to the left and right",
                        x, y
                    )));
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Wall {
    pub left_bottom: Point2<i8>,
    pub right_top: Point2<i8>,
}

/// A [`Grid`] with precomputed data for faster pathfinding.
///
/// This struct is created by [`ComputedGrid::try_from`].
///
/// # Examples
///
/// ```
/// use mdrc_pacbot_util::grid::ComputedGrid;
/// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
///
/// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputedGrid {
    grid: Grid,

    pellet_count: u32,
    power_pellets: Vec<Point2<u8>>,

    walkable_nodes: Vec<Point2<u8>>,
    coords_to_node: HashMap<Point2<u8>, usize>,

    /// walkable, right, left, up, down
    valid_actions: Vec<[bool; 5]>,
    /// note that all walkable nodes might not be reachable from each other
    distance_matrix: Vec<Vec<Option<u8>>>,

    /// walls represent rectangles with top left corner at the specified point
    walls: Vec<Wall>,
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
        // iterate through all grid positions
        for y in 1..GRID_HEIGHT - 1 {
            for x in 1..GRID_WIDTH - 1 {
                let pos = Point2::new(x as u8, y as u8);
                let tile = grid[x][y];
                if tile == GridValue::o {
                    // count pellets
                    pellet_count += 1;
                } else if tile == GridValue::O {
                    // remember super pellets
                    power_pellets.push(pos);
                }
                if tile.walkable() {
                    // remember walkable nodes
                    let node_index = walkable_nodes.len();
                    walkable_nodes.push(pos);
                    coords_to_node.insert(pos, node_index);
                    // quick lookup for whether a node is walkable in a given direction
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

        // initialize distance matrix
        for _ in 0..walkable_nodes.len() {
            distance_matrix.push(vec![None; walkable_nodes.len()]);
        }

        // initialize ComputedGrid
        let mut s = ComputedGrid {
            grid,
            pellet_count,
            power_pellets,
            walkable_nodes,
            coords_to_node,
            valid_actions,
            distance_matrix,
            walls: Vec::new(),
        };

        // compute distance matrix with BFS
        for (i, &start) in s.walkable_nodes.iter().enumerate() {
            let mut visited = vec![false; s.walkable_nodes.len()];
            let mut queue = vec![(start, 0)];
            while let Some((pos, dist)) = queue.pop() {
                // only walkable nodes are added to the queue
                let node_index = *s.coords_to_node.get(&pos).unwrap();
                if visited[node_index] {
                    continue;
                }
                visited[node_index] = true;
                s.distance_matrix[i][node_index] = Some(dist);
                for neighbor in s.neighbors(&pos) {
                    queue.push((neighbor, dist + 1));
                }
            }
        }

        fn is_wall(g: &ComputedGrid, p: &Point2<i8>) -> bool {
            let parts = [
                Point2::new(p.x, p.y),
                Point2::new(p.x + 1, p.y),
                Point2::new(p.x, p.y + 1),
                Point2::new(p.x + 1, p.y + 1),
            ];
            parts.iter().all(|part| {
                if part.x < 0 || part.y < 0 {
                    return true;
                }
                let part_u8 = Point2::new(part.x as u8, part.y as u8);
                g.at(&part_u8).is_none() || !g.at(&part_u8).unwrap().walkable()
            })
        }

        let mut x = -1i8;
        let mut y = -1i8;
        loop {
            // make sure this point isn't already a part of a wall
            let mut is_part_of_wall = false;
            for wall in &s.walls {
                if wall.left_bottom.x <= x
                    && wall.left_bottom.y <= y
                    && wall.right_top.x > x
                    && wall.right_top.y > y
                {
                    is_part_of_wall = true;
                    break;
                }
            }
            // compute walls - first, add each cell individually
            if !is_part_of_wall && is_wall(&s, &Point2::new(x, y)) {
                let mut wall = Wall {
                    left_bottom: Point2::new(x, y),
                    right_top: Point2::new(x + 1, y + 1),
                };

                x += 1;

                // extend the wall to the right
                while is_wall(&s, &Point2::new(x, y)) {
                    wall.right_top.x += 1;
                    x += 1;

                    if x >= GRID_WIDTH as i8 {
                        break;
                    }
                }

                // Extend the wall up
                let mut next_y = y + 1;
                while next_y < GRID_HEIGHT as i8 {
                    let mut can_extend = true;
                    for next_x in wall.left_bottom.x..wall.right_top.x {
                        if !is_wall(&s, &Point2::new(next_x, next_y)) {
                            can_extend = false;
                            break;
                        }
                    }
                    if can_extend {
                        wall.right_top.y += 1;
                        next_y += 1;
                    } else {
                        break;
                    }
                }

                s.walls.push(wall);
            } else {
                x += 1;
            }

            if x >= GRID_WIDTH as i8 {
                x = -1i8;
                y += 1;

                if y == GRID_HEIGHT as i8 {
                    break;
                }
            }
        }

        Ok(s)
    }
}

impl ComputedGrid {
    /// Returns the underlying [`Grid`].
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    ///
    /// assert_eq!(grid.grid()[0][0], mdrc_pacbot_util::grid::GridValue::I);
    /// ```
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Returns the number of pellets in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.pellet_count(), 0);
    /// ```
    pub fn pellet_count(&self) -> u32 {
        self.pellet_count
    }

    /// Returns the positions of all power pellets in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert!(grid.power_pellets().is_empty());
    /// ```
    pub fn power_pellets(&self) -> &Vec<Point2<u8>> {
        &self.power_pellets
    }

    /// Returns the positions of all walkable nodes in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.walkable_nodes()[0], Point2::new(1, 1));
    /// ```
    pub fn walkable_nodes(&self) -> &Vec<Point2<u8>> {
        &self.walkable_nodes
    }

    /// Returns the index of the given position in the [`walkable_nodes`] vector, or `None` if the
    /// position is not walkable.
    ///     
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.coords_to_node(&Point2::new(1, 1)), Some(0));
    /// assert_eq!(grid.coords_to_node(&Point2::new(0, 0)), None);
    /// ```
    pub fn coords_to_node(&self, p: &Point2<u8>) -> Option<usize> {
        self.coords_to_node.get(p).copied()
    }

    /// Returns the valid actions for the given position.
    ///
    /// The five values represent:
    /// - whether the node is walkable
    /// - whether the node to the right is walkable
    /// - whether the node to the left is walkable
    /// - whether the node above is walkable
    /// - whether the node below is walkable
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.valid_actions(Point2::new(1, 1)), Some([true, false, false, false, false]));
    /// ```
    pub fn valid_actions(&self, p: Point2<u8>) -> Option<[bool; 5]> {
        let node_index = self.coords_to_node.get(&p)?;
        Some(self.valid_actions[*node_index])
    }

    /// Returns the [`GridValue`] at the given position, or `None` if the position is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.at(&Point2::new(0, 0)), Some(mdrc_pacbot_util::grid::GridValue::I));
    /// assert_eq!(grid.at(&Point2::new(1, 1)), Some(mdrc_pacbot_util::grid::GridValue::e));
    /// assert_eq!(grid.at(&Point2::new(32, 32)), None);
    /// ```
    pub fn at(&self, p: &Point2<u8>) -> Option<GridValue> {
        if p.x >= GRID_WIDTH as u8 || p.y >= GRID_HEIGHT as u8 {
            return None;
        }
        Some(self.grid[p.x as usize][p.y as usize])
    }

    /// Returns the [`Point`] in the given direction from the given position, or `None` if the
    /// position is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::{ComputedGrid, Direction};
    /// use mdrc_pacbot_util::standard_grids::GRID_BLANK;
    ///
    /// let grid = ComputedGrid::try_from(GRID_BLANK).unwrap();
    /// assert_eq!(grid.next(&Point2::new(1, 1), &Direction::Right), Some(Point2::new(2, 1)));
    /// assert_eq!(grid.next(&Point2::new(1, 1), &Direction::Left), Some(Point2::new(0, 1)));
    /// assert_eq!(grid.next(&Point2::new(1, 1), &Direction::Up), Some(Point2::new(1, 2)));
    /// assert_eq!(grid.next(&Point2::new(1, 1), &Direction::Down), Some(Point2::new(1, 0)));
    /// ```
    pub fn next(&self, p: &Point2<u8>, direction: &Direction) -> Option<Point2<u8>> {
        match direction {
            Direction::Right => {
                if p.x == GRID_WIDTH as u8 - 1 {
                    return None;
                }
                Some(Point2::new(p.x + 1, p.y))
            }
            Direction::Left => {
                if p.x == 0 {
                    return None;
                }
                Some(Point2::new(p.x - 1, p.y))
            }
            Direction::Up => {
                if p.y == GRID_HEIGHT as u8 - 1 {
                    return None;
                }
                Some(Point2::new(p.x, p.y + 1))
            }
            Direction::Down => {
                if p.y == 0 {
                    return None;
                }
                Some(Point2::new(p.x, p.y - 1))
            }
        }
    }

    /// Returns the distance between two points, or `None` if the points are not both walkable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_PACMAN;
    ///
    /// let grid = ComputedGrid::try_from(GRID_PACMAN).unwrap();
    /// assert_eq!(grid.dist(&Point2::new(1, 1), &Point2::new(1, 1)), Some(0));
    /// assert_eq!(grid.dist(&Point2::new(1, 1), &Point2::new(1, 2)), Some(1));
    /// ```
    pub fn dist(&self, p1: &Point2<u8>, p2: &Point2<u8>) -> Option<u8> {
        let p1 = self.coords_to_node.get(p1)?;
        let p2 = self.coords_to_node.get(p2)?;
        self.distance_matrix[*p1][*p2]
    }

    /// Returns all the walkable neighbors of the given position.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_PACMAN;
    ///
    /// let grid = ComputedGrid::try_from(GRID_PACMAN).unwrap();
    /// assert!(grid.neighbors(&Point2::new(1, 1)).contains(&Point2::new(1, 2)));
    /// assert!(grid.neighbors(&Point2::new(1, 1)).contains(&Point2::new(2, 1)));
    /// ```
    pub fn neighbors(&self, p: &Point2<u8>) -> Vec<Point2<u8>> {
        let mut neighbors = vec![];
        for &neighbor in &[
            Point2::new(p.x + 1, p.y),
            Point2::new(p.x - 1, p.y),
            Point2::new(p.x, p.y + 1),
            Point2::new(p.x, p.y - 1),
        ] {
            if let Some(grid_value) = self.at(&neighbor) {
                if grid_value.walkable() {
                    neighbors.push(neighbor);
                }
            }
        }
        neighbors
    }

    /// Returns the [`Wall`]s in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::Point2;
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::standard_grids::GRID_PACMAN;
    ///
    /// let grid = ComputedGrid::try_from(GRID_PACMAN).unwrap();
    /// let walls = grid.walls();
    /// ```
    pub fn walls(&self) -> &Vec<Wall> {
        &self.walls
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
        assert_eq!(
            grid.next(&Point2::new(1, 1), &Direction::Right),
            Some(Point2::new(2, 1))
        );
        assert_eq!(
            grid.next(&Point2::new(1, 1), &Direction::Left),
            Some(Point2::new(0, 1))
        );
        assert_eq!(
            grid.next(&Point2::new(1, 1), &Direction::Up),
            Some(Point2::new(1, 2))
        );
        assert_eq!(
            grid.next(&Point2::new(1, 1), &Direction::Down),
            Some(Point2::new(1, 0))
        );
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
