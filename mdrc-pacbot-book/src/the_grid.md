# The Grid

## The Logical Grid

The logical grid represents the pathways where Pacman and the ghost can travel in integer coordinates.

The actual Pacbot grid is 28 cells wide by 31 cells tall, but this code supports 32x32 grids.

## Coordinates

+x = right

+y = up

Points are always given as (x, y). Both x and y are represented as u8 to save space.
The 2-dimensional list that represents the grid should be indexed like `grid[x][y]`. 
To avoid confusion, instead of indexing directly, it is recommended to only use 
`ComputedGrid.at()` with a `Point2`.

Note that the array looks sideways when viewed in an editor:
```ignore
[
    [bottom left, ..., top left],
    ...,
    [bottom right, ..., top right]
]
```
When displayed in a running program, (0, 0) should always be in the bottom left. 
This matches the official visualization from Harvard Robotics.

So this grid:
```ignore
[
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
]
```
Would appear like this:
```ignore
2 5 8
1 4 7
0 3 6
```

On the official Pacbot grid:
- (1, 1) is the bottom left corner
- (26, 1) is the bottom right corner
- (1, 29) is the top left corner
- (26, 29) is the top right corner

Pacbot's starting position is (14, 7).