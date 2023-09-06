# Grid & Coordinate System

## `Grid`

Pacman is played on a 2D integer grid, meaning the location of all objects 
(including walls, ghosts, pellets, and Pacman) can be fully described with 
two integers `x` and `y`. 

The `Grid` data type, an alias for `Vec<Vec<GridValue>>` provides information about 
the state of a Pacman game before Pacman or any ghosts have moved, including:
- Location of walls
- Location of pellets
- Location of super pellets
- Ghost waiting area location

Note that `Grid` does not contain information about Pacman or ghost starting locations or paths.

`GridValue` can be any of the following:
- `I`: Wall - not walkable for Pacman and Ghosts
- `o`: Pellet
- `e`: Empty (walkable)
- `O`: Super pellet
- `n`: Ghost chambers - not walkable for Pacman
- `c`: Cherry position

A coordinate is "walkable" for some entity if the entity is able to travel there.

`Grid` is `GRID_WIDTH` x `GRID_HEIGHT` size. The official Pacbot grid
is 28 cells wide by 31 cells tall, but this code currently supports up to 32 x 32 grids.
Any coordinate less than (0, 0) or greater than (31, 31) is treated as a wall, and is not stored in `Grid`.

## Standard Grids

A number of standard grids are provided in `mdrc_pacbot_util::standard_grids`. These include several
common configurations:
- `GRID_PACMAN` - The official Pacbot `Grid`
- `GRID_BLANK` - A `Grid` entirely composed of walls (except for the space at (1, 1)), copy-paste-able to create new `Grid`s
- `GRID_OUTER` - A `Grid` with an empty pathway around the inside of the outer edge
- `GRID_PLAYGROUND` - A `Grid` with many small areas for testing motor control algorithms

These can be used as-is or edited to create custom `Grid`s. 
Additionally, you can use any `Vec<Vec<GridValue>>` of the appropriate size.

### Upgrading to `ComputedGrid`

The [ComputedGrid](./computed_grid.md) struct provides additional pre-calculated information about `Grid`s.
The section [Grid Rules](./computed_grid.md#grid-rules) contains more information about special rules, like that a `Grid` must have 
Walls around the entire outer edge, among others.

Once a `Grid` is upgraded to a `ComputedGrid` via `ComputedGrid::try_from(grid)`, which is necessary for many other parts of this library, it becomes
immutable.

## Coordinates

We have chosen the following coordinate system to fit best with the official Pacbot codebase.

`+x` is "right" and `+y` is "up", where (0, 0) is the origin and (31, 31) 
is the farthest point from the origin:

```
             (31, 31)
 ^
 |
+y   (...)

(0,0)   +x -->
```

However, `Grid` is indexed like `grid[x][y]`. As a consequence, in the code, `Grid` appears sideways.

This code layout:
```
[              +y ->
     +x    [a, b, b, b],
     |     [a, b, b, b],
     v     [a, b, b, b],
];
```

Would appear like this:
```
angle = 90 degrees, or pi/2

 ^   b b b
 |   b b b
 +y  b b b
     a a a
       +x ->    angle = 0
```

When looking at code, imagine turning the grid 90 degrees counter-clockwise.

On the official Pacbot grid:

- (1, 1) is the bottom left walkable corner
- (26, 1) is the bottom right walkable corner
- (1, 29) is the top left walkable corner
- (26, 29) is the top right walkable corner

Pacbot's starting position is (14, 7).

## Physical Coordinates

The integer coordinate system described above is easily extendable to a physical floating point
coordinate system.

When points are given as floating point values, integer coordinates (like 1.0, 1.0) represent
the center of the corresponding `Grid` cell (in this case, (1, 1)).

Note that walls in the physical Pacbot grid are "eroded" by half a cell so that they are two
`Grid` cells wide. This is why, except for the outer edge, there can't be a wall that is one cell 
wide - both sides would get eroded by half a cell and there would be an infinitely thin wall.

Curiously, this results in the edges of walls falling on integer coordinates that lie at the center
of the `Grid` cells that are declared as walls.

## Angles

An angle of 0 degrees corresponds to the `+x` direction. As the angle increases in the positive direction, it rotates
counter-clockwise, with 90 degrees pointing towards the `+y` direction.



