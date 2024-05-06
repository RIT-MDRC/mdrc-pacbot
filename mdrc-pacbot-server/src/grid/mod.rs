//! Logical grid structs and utilities.

use crate::grid::standard_grids::StandardGrid;
use nalgebra::Point2;
use ordered_float::OrderedFloat;
use pacbot_rs::location::{DOWN, LEFT, RIGHT, UP};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[rustfmt::skip]
pub mod standard_grids;

/// Width of a [`Grid`].
pub const GRID_COLS: usize = 32;
/// Height of a [`Grid`].
pub const GRID_ROWS: usize = 32;

/// A 2D grid
///
/// The grid is indexed by `grid[row][col]`
pub type Grid = [[bool; GRID_COLS]; GRID_ROWS];

/// Validates a [`Grid`].
///
/// A valid [`Grid`] must satisfy the following conditions:
/// - The edges of the grid must all be walls.
/// - There must be no 2x2 walkable squares.
/// - There must be at least one walkable space.
/// - No wall should have a walkable cell either both above and below or both to the left and right
fn validate_grid(grid: &Grid) -> Result<(), String> {
    // the edges of the grid should all be walls
    if (0..GRID_ROWS).any(|row| !grid[row][0]) {
        return Err("Left edge of grid is not all walls".to_string());
    }
    if (0..GRID_ROWS).any(|row| !grid[row][GRID_COLS - 1]) {
        return Err("Right edge of grid is not all walls".to_string());
    }
    if (0..GRID_COLS).any(|col| !grid[0][col]) {
        return Err("Top edge of grid is not all walls".to_string());
    }
    if (0..GRID_COLS).any(|col| !grid[GRID_ROWS - 1][col]) {
        return Err("Bottom edge of grid is not all walls".to_string());
    }

    // there should be no 2x2 walkable squares
    for row in 0..GRID_ROWS - 1 {
        for col in 0..GRID_COLS - 1 {
            if !grid[row][col]
                && !grid[row][col + 1]
                && !grid[row + 1][col]
                && !grid[row + 1][col + 1]
            {
                return Err(format!("2x2 walkable square at ({}, {})", row, col));
            }
        }
    }

    // there should be at least one walkable space
    if grid.iter().all(|row| row.iter().all(|wall| *wall)) {
        return Err("No walkable spaces".to_string());
    }

    // no wall should have a walkable cell either both above and below or both to the left and right
    for row in 1..GRID_ROWS - 1 {
        for col in 1..GRID_COLS - 1 {
            if grid[row][col] {
                if !grid[row - 1][col] && !grid[row + 1][col] {
                    return Err(format!(
                        "Wall at ({}, {}) has walkable cells both above and below",
                        row, col
                    ));
                }
                if !grid[row][col - 1] && !grid[row][col + 1] {
                    return Err(format!(
                        "Wall at ({}, {}) has walkable cells both to the left and right",
                        row, col
                    ));
                }
            }
        }
    }

    Ok(())
}

/// A rectangle representing a wall.
///
/// The rectangle is defined by the top left corner and the bottom right corner.
/// Note that [`Wall`] does not follow the same grid conventions as [`Grid`].
/// The coordinates are intended to be +0.5, and may be negative.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    /// The top left corner of the [`Wall`].
    pub top_left: Point2<i8>,
    /// The bottom right corner of the [`Wall`].
    pub bottom_right: Point2<i8>,
}

/// A [`Grid`] with precomputed data for faster pathfinding.
///
/// This struct is created by [`ComputedGrid::try_from`].
///
/// # Examples
///
/// ```
/// use mdrc_pacbot_util::grid::ComputedGrid;
/// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
///
/// let grid = StandardGrid::Blank.compute_grid();
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputedGrid {
    grid: Grid,
    standard_grid: Option<StandardGrid>,

    walkable_nodes: Vec<Point2<i8>>,
    coords_to_node: HashMap<Point2<i8>, usize>,

    /// walkable, right, left, up, down
    valid_actions: Vec<[bool; 5]>,
    /// note that all walkable nodes might not be reachable from each other
    distance_matrix: Vec<Vec<Option<u8>>>,

    /// walls represent rectangles with top left corner at the specified point
    walls: Vec<Wall>,
}

impl Default for ComputedGrid {
    fn default() -> Self {
        standard_grids::StandardGrid::Pacman.compute_grid()
    }
}

impl TryFrom<Grid> for ComputedGrid {
    type Error = String;

    fn try_from(grid: Grid) -> Result<Self, Self::Error> {
        validate_grid(&grid)?;

        let mut walkable_nodes = vec![];
        let mut coords_to_node: HashMap<Point2<i8>, usize> = HashMap::new();

        let mut valid_actions = vec![];
        let mut distance_matrix = vec![];

        // note that all edges must be walls
        // iterate through all grid positions
        for row in 1..GRID_ROWS - 1 {
            for col in 1..GRID_COLS - 1 {
                let pos = Point2::new(row as i8, col as i8);
                if !grid[row][col] {
                    // remember walkable nodes
                    let node_index = walkable_nodes.len();
                    walkable_nodes.push(pos);
                    coords_to_node.insert(pos, node_index);
                    // quick lookup for whether a node is walkable in a given direction
                    valid_actions.push([
                        true,
                        !grid[row - 1][col],
                        !grid[row][col - 1],
                        !grid[row + 1][col],
                        !grid[row][col + 1],
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
            standard_grid: None,
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
                    true
                } else {
                    g.wall_at(part)
                }
            })
        }

        fn is_part_of_wall(g: &ComputedGrid, p: &Point2<i8>) -> bool {
            for wall in &g.walls {
                if p.x >= wall.top_left.x
                    && p.y >= wall.top_left.y
                    && p.x < wall.bottom_right.x
                    && p.y < wall.bottom_right.y
                {
                    return true;
                }
            }
            false
        }

        let mut row = -1;
        let mut col = -1;
        loop {
            // make sure this point isn't already a part of a wall
            let is_already_wall = is_part_of_wall(&s, &Point2::new(row, col));
            // compute walls - first, add each cell individually
            if !is_already_wall && is_wall(&s, &Point2::new(row, col)) {
                let mut wall = Wall {
                    top_left: Point2::new(row, col),
                    bottom_right: Point2::new(row + 1, col + 1),
                };

                if wall.top_left.y > GRID_COLS as i8 {
                    wall.top_left.y = GRID_COLS as i8;
                }

                col += 1;

                // extend the wall to the right
                while is_wall(&s, &Point2::new(row, col))
                    && !is_part_of_wall(&s, &Point2::new(row, col))
                {
                    if col >= GRID_COLS as i8 {
                        break;
                    }

                    wall.bottom_right.y += 1;
                    col += 1;
                }

                // Extend the wall down
                let mut next_row = row + 1;
                while next_row < GRID_ROWS as i8 {
                    let mut can_extend = true;
                    for next_col in wall.top_left.y..wall.bottom_right.y {
                        if !is_wall(&s, &Point2::new(next_row, next_col))
                            || is_part_of_wall(&s, &Point2::new(next_row, next_col))
                        {
                            can_extend = false;
                            break;
                        }
                    }
                    if can_extend {
                        wall.bottom_right.x += 1;
                        next_row += 1;
                    } else {
                        break;
                    }
                }

                s.walls.push(wall);
            } else {
                col += 1;
            }

            if col >= GRID_COLS as i8 {
                col = -1;
                row += 1;

                if row == GRID_ROWS as i8 {
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
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Blank.compute_grid();
    ///
    /// assert_eq!(grid.grid()[0][0], true);
    /// ```
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Returns the underlying [`StandardGrid`], if one was used to construct it.
    pub fn standard_grid(&self) -> &Option<StandardGrid> {
        &self.standard_grid
    }

    /// Returns the positions of all walkable nodes in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Blank.compute_grid();
    /// assert_eq!(grid.walkable_nodes()[0], IntLocation::new(1, 1));
    /// ```
    pub fn walkable_nodes(&self) -> &Vec<Point2<i8>> {
        &self.walkable_nodes
    }

    /// Returns the index of the given position in the walkable_nodes vector, or `None` if the
    /// position is not walkable.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Blank.compute_grid();
    /// assert_eq!(grid.coords_to_node(&IntLocation::new(1, 1)), Some(0));
    /// assert_eq!(grid.coords_to_node(&IntLocation::new(0, 0)), None);
    /// ```
    pub fn coords_to_node(&self, p: &Point2<i8>) -> Option<usize> {
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
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Blank.compute_grid();
    /// assert_eq!(grid.valid_actions(IntLocation::new(1, 1)), Some([true, false, false, false, false]));
    /// ```
    pub fn valid_actions(&self, p: Point2<i8>) -> Option<[bool; 5]> {
        let node_index = self.coords_to_node.get(&p)?;
        Some(self.valid_actions[*node_index])
    }

    /// Returns whether there is a wall at a given position
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Blank.compute_grid();
    /// assert_eq!(grid.wall_at(&IntLocation::new(0, 0)), true);
    /// assert_eq!(grid.wall_at(&IntLocation::new(1, 1)), false);
    /// assert_eq!(grid.wall_at(&IntLocation::new(32, 32)), true);
    /// ```
    pub fn wall_at(&self, p: &Point2<i8>) -> bool {
        if p.x >= GRID_ROWS as i8 || p.y >= GRID_COLS as i8 || p.x < 0 || p.y < 0 {
            true
        } else {
            self.grid[p.x as usize][p.y as usize]
        }
    }

    /// Returns the distance between two points, or `None` if the points are not both walkable.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Pacman.compute_grid();
    /// assert_eq!(grid.dist(&IntLocation::new(1, 1), &IntLocation::new(1, 1)), Some(0));
    /// assert_eq!(grid.dist(&IntLocation::new(1, 1), &IntLocation::new(1, 2)), Some(1));
    /// ```
    pub fn dist(&self, p1: &Point2<i8>, p2: &Point2<i8>) -> Option<u8> {
        let p1 = self.coords_to_node.get(p1)?;
        let p2 = self.coords_to_node.get(p2)?;
        self.distance_matrix[*p1][*p2]
    }

    /// Returns all the walkable neighbors of the given position.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::{ComputedGrid, IntLocation};
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Pacman.compute_grid();
    /// assert!(grid.neighbors(&IntLocation::new(1, 1)).contains(&IntLocation::new(1, 2)));
    /// assert!(grid.neighbors(&IntLocation::new(1, 1)).contains(&IntLocation::new(2, 1)));
    /// ```
    pub fn neighbors(&self, p: &Point2<i8>) -> Vec<Point2<i8>> {
        let mut neighbors = vec![];
        let mut potential_neighbors = vec![Point2::new(p.x + 1, p.y), Point2::new(p.x, p.y + 1)];
        if p.x > 0 {
            potential_neighbors.push(Point2::new(p.x - 1, p.y));
        }
        if p.y > 0 {
            potential_neighbors.push(Point2::new(p.x, p.y - 1));
        }
        for &neighbor in &potential_neighbors {
            if !self.wall_at(&neighbor) {
                neighbors.push(neighbor);
            }
        }
        neighbors
    }

    /// Returns the [`Wall`]s in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::grid::ComputedGrid;
    /// use mdrc_pacbot_util::grid::standard_grids::StandardGrid;
    ///
    /// let grid = StandardGrid::Pacman.compute_grid();
    /// let walls = grid.walls();
    /// ```
    pub fn walls(&self) -> &Vec<Wall> {
        &self.walls
    }

    /// Return the walkable node from the nodes surrounding this point
    pub fn node_nearest(&self, x: f32, y: f32) -> Option<Point2<i8>> {
        [
            Point2::new(x.floor() as i8, y.floor() as i8),
            Point2::new(x.ceil() as i8, y.floor() as i8),
            Point2::new(x.floor() as i8, y.ceil() as i8),
            Point2::new(x.ceil() as i8, y.ceil() as i8),
        ]
        .into_iter()
        .filter(|&node| !self.wall_at(&node))
        .min_by_key(|&node| {
            let dx = node.x as f32 - x;
            let dy = node.y as f32 - y;
            OrderedFloat::from(dx * dx + dy * dy)
        })
    }

    /// Returns the shortest path, if one exists, from start to finish
    /// The path includes path the start and the finish
    pub fn bfs_path(&self, start: Point2<i8>, finish: Point2<i8>) -> Option<Vec<Point2<i8>>> {
        let mut prev: HashMap<Point2<i8>, Option<Point2<i8>>> = HashMap::new();
        let mut queue: VecDeque<Point2<i8>> = VecDeque::new();
        prev.insert(start, None);
        queue.push_back(start);
        while let Some(current) = queue.pop_front() {
            if current == finish {
                let mut path = vec![finish];
                let mut next = finish;
                while let Some(Some(before_next)) = prev.get(&next) {
                    path.insert(0, *before_next);
                    next = *before_next;
                }
                return Some(path);
            }
            let neighbors = self.neighbors(&current);
            for n in &neighbors {
                if prev.get(n).is_none() {
                    prev.insert(*n, Some(current));
                    queue.push_back(*n);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::standard_grids::*;

    #[test]
    fn valid_preset_grids() {
        assert!(validate_grid(&GRID_PACMAN).is_ok());
        assert!(validate_grid(&GRID_BLANK).is_ok());
    }

    #[test]
    fn validation_require_empty_space() {
        let mut grid = GRID_BLANK;
        grid[1][1] = true;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(format!("{}", v.unwrap_err()), "No walkable spaces");
    }

    #[test]
    fn validation_invalid_bottom_wall() {
        let mut grid = GRID_BLANK;
        grid[GRID_ROWS - 1][1] = false;

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
        grid[0][1] = false;

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
        grid[1][0] = false;

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
        grid[1][GRID_COLS - 1] = false;

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
        grid[1][1] = false;
        grid[1][2] = false;
        grid[2][1] = false;
        grid[2][2] = false;

        let v = validate_grid(&grid);
        assert!(v.is_err());
        assert_eq!(
            format!("{}", v.unwrap_err()),
            "2x2 walkable square at (1, 1)"
        );
    }

    #[test]
    fn compute_preset_grids() {
        StandardGrid::Pacman.compute_grid();
        StandardGrid::Blank.compute_grid();
    }

    #[test]
    fn compute_walkable_nodes() {
        let mut grid = GRID_BLANK;
        grid[1][1] = false;
        grid[1][2] = false;
        grid[6][1] = false;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.walkable_nodes.len(), 3);
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(1, 1)));
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(1, 2)));
        assert!(computed_grid.walkable_nodes.contains(&Point2::new(6, 1)));
    }

    #[test]
    fn compute_coords_to_node() {
        let mut grid = GRID_BLANK;
        grid[1][1] = false;
        grid[1][2] = false;
        grid[6][1] = false;

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
        grid[1][1] = false;
        grid[1][2] = false;
        grid[6][1] = false;

        let computed_grid = ComputedGrid::try_from(grid).unwrap();
        assert_eq!(computed_grid.valid_actions.len(), 3);
        let one_one_idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(1, 1))
            .unwrap();
        assert_eq!(
            computed_grid.valid_actions[one_one_idx],
            [true, false, false, false, true]
        );

        let one_two_idx = *computed_grid
            .coords_to_node
            .get(&Point2::new(1, 2))
            .unwrap();
        assert_eq!(
            computed_grid.valid_actions[one_two_idx],
            [true, false, true, false, false]
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
        grid[1][1] = false;
        grid[1][2] = false;
        grid[6][1] = false;

        let points = [Point2::new(1, 1), Point2::new(1, 2), Point2::new(6, 1)];

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
    fn grid_at() {
        let grid = StandardGrid::Blank.compute_grid();
        assert_eq!(grid.wall_at(&Point2::new(0, 0)), true);
    }

    #[test]
    fn grid_at_oob() {
        let grid = StandardGrid::Blank.compute_grid();
        assert_eq!(grid.wall_at(&Point2::new(0, GRID_ROWS as i8)), true);
        assert_eq!(grid.wall_at(&Point2::new(GRID_COLS as i8, 0)), true);
    }
}

/// Find the direction from the start point to the end point
pub fn facing_direction(start: &Point2<i8>, end: &Point2<i8>) -> u8 {
    if start.y > end.y {
        RIGHT
    } else if start.y < end.y {
        LEFT
    } else if start.x < end.x {
        UP
    } else if start.x > end.x {
        DOWN
    } else {
        // start == end
        RIGHT
    }
}
