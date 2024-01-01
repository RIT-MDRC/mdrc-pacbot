# Grid & Coordinate System

## `Grid`

Pacman is played on a 2D integer grid, meaning the location of all objects 
(including walls, ghosts, pellets, and Pacman) can be fully described with 
two integers `row` and `col`. 

The `Grid` data type, an alias for `[[bool; GRID_COLS]; GRID_ROWS]` provides information about which cells are walls.

A coordinate is "walkable" for some entity if the entity is able to travel there.

`Grid` is `GRID_ROWS` x `GRID_COLS` size. The official Pacbot grid
is 28 cells wide by 31 cells tall, but this code currently supports up to 32 x 32 grids.
Any coordinate less than (0, 0) or greater than (31, 31) is treated as a wall, and is not stored in `Grid`.

## Standard Grids

A number of standard grids are provided in `mdrc_pacbot_util::grid::standard_grids`. These include several
common configurations:
- `GRID_PACMAN` - The official Pacbot `Grid`
- `GRID_BLANK` - A `Grid` entirely composed of walls (except for the space at (1, 1)), copy-paste-able to create new `Grid`s
- `GRID_OUTER` - A `Grid` with an empty pathway around the inside of the outer edge
- `GRID_PLAYGROUND` - A `Grid` with many small areas for testing motor control algorithms

These can be used as-is or edited to create custom `Grid`s. 
Additionally, you can use any `[[bool; GRID_COLS]; GRID_ROWS]` of the appropriate size.

### Upgrading to `ComputedGrid`

The [ComputedGrid](./computed_grid.md) struct provides additional pre-calculated information about `Grid`s.
The section [Grid Rules](./computed_grid.md#grid-rules) contains more information about special rules, like that a `Grid` must have 
Walls around the entire outer edge, among others.

Once a `Grid` is upgraded to a `ComputedGrid` via `ComputedGrid::try_from(grid)`, which is necessary for many other parts of this library, it becomes
immutable.

## Coordinates

We have chosen the following coordinate system to fit best with the official Pacbot codebase.

In place of `x` and `y`, we use `row` and `col`.

`row` is increasing in the "down" direction and `col` is increasing in the "up" direction, 
where (0, 0) is the origin and (31, 31) is the farthest point from the origin:

```ignore
(0, 0)    --> +col
       
 |    (...)
 v
+row       (31, 31)
```

When they appear as a pair, the order is always (`row`, `col`).

If they must be translated to `x` and `y` (for example, for angle calculations) then `row` is `x` and `col` is `y`.
This means that angle 0 is downwards.

On the official Pacbot grid:

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

## Angles

An angle of 0 degrees corresponds to the `+row` direction, or "down". As the angle increases in the positive direction, 
it rotates counter-clockwise, with 90 degrees pointing towards the `+col` direction.



