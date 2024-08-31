# Grid & Coordinate System

## `Grid`

Pacman is played on a 2D integer grid, meaning the location of all objects 
(including walls, ghosts, pellets, and Pacman) can be fully described with 
two integers. 

The `Grid` data type, an alias for `[[bool; GRID_COLS]; GRID_ROWS]` provides information about which cells are walls.

`Grid` is `GRID_ROWS` x `GRID_COLS` size. The official Pacbot grid
is 28 cells 31 cells, but this code currently supports up to 32 x 32 grids.
Any coordinate less than (0, 0) or greater than (31, 31) is treated as a wall, and is not stored in `Grid`.

## Standard Grids

A number of standard grids are provided in `core_pb::grid::standard_grid`. These include several
common configurations:
- `GRID_PACMAN` - The official Pacbot `Grid`
- `GRID_BLANK` - A `Grid` entirely composed of walls (except for the space at (1, 1)), copy-paste-able to create new `Grid`s
- `GRID_OUTER` - A `Grid` with an empty pathway around the inside of the outer edge
- `GRID_PLAYGROUND` - A `Grid` with many small areas for testing motor control algorithms

These can be used as-is or edited to create custom `Grid`s. However, most parts of the application do not support `Grid`s
outside the `StandardGrid`s.

### Upgrading to `ComputedGrid`

The [ComputedGrid](./computed_grid.md) struct provides additional pre-calculated information about `Grid`s.
The section [Grid Rules](./computed_grid.md#grid-rules) contains more information about special rules, like that a `Grid` must have 
Walls around the entire outer edge, among others.

Once a `Grid` is upgraded to a `ComputedGrid` via `ComputedGrid::try_from(grid)`, which is necessary for many other parts of this library, it becomes
immutable.

## Coordinates

Throughout `mdrc_pacbot`, we use `nalgebra::Point2` as much as possible to describe location. This has two properties,
`x` and `y`. *Most of the time*, we use the same coordinate system you've used in math class for many years:

```ignore
+y; 90°     (31, 31)
 🡑     
 |     (...)
 |
(0, 0)-----🡒 +x; 0°
```

where `+x` and 0° is to the right, `+y` is up, and angle is more positive in the counter-clockwise direction. Using
this system, angle functions like sin and cos behave as expected.

The only downside to this system is that it differs from the official Pacbot Pac-Man implementation. There, they
use `row` and `col`, with coordinates in the form `(row, col)`, with the intended orientation like so:

```ignore
(0, 0)    --> +col
       
 |    (...)
 v
+row       (31, 31)
```

We choose not to do this because it places (0, 0) at the top left which makes physics calculations confusing. So we
rotate the official game by 90° and translate `(row, col)` to `(x, y)`. If you want to view the grid without the rotation,
click the "Rotated Grid" checkbox in the gui.

On the official Pacbot grid (in either rotation):

- (1, 1)   is the top    left  walkable corner
- (1, 26)  is the top    right walkable corner
- (29, 1)  is the bottom left  walkable corner
- (29, 26) is the top    right walkable corner

Pacbot's starting position is (23, 13), facing right.

## Physical Coordinates

The integer coordinate system described above is easily extendable to a physical floating point
coordinate system.

When points are given as floating point values, integer coordinates (like 1.0, 1.0) represent
the center of the corresponding `Grid` cell (in this case, (1, 1)).

Note that walls in the physical Pacbot grid are "eroded" by half a cell so that paths are two
`Grid` cells wide. This is why, except for the outer edge, there can't be a wall in `Grid` that is one cell 
wide - both sides would get eroded by half a cell and there would be an infinitely thin wall.

Curiously, this results in the edges of walls falling on integer coordinates that lie at the center
of the `Grid` cells that are declared as walls.



