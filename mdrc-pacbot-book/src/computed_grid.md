# Computed Grid

`ComputedGrid` provides additional information about a `Grid` that is calculated when 
the `ComputedGrid` is first created, meaning it is very cheap to retrieve.

For this documentation, a "walkable space" is one where Pacman can travel.

## Grid Rules

In order for a `Grid` to be successfully upgraded to a `ComputedGrid` via `ComputedGrid::try_from(grid)`,
all of the following must be true:
- All the cells around the outside of the grid (with `x` or `y` equal to 0 or 31) are walls
- There is at least one walkable cell (where Pacman can spawn)
- There are no 2x2 empty squares
- There is no wall with a walkable space both above and below it
- There is no wall with a walkable space both to the left and right

The last two points can be simplified to say that all walls are 2 cells thick.

Note:
- It is not necessary that a `ComputedGrid` is connected, or that every walkable space is accessible from every other one
- It is not necessary that there is are ghost chambers, pellets, or super pellets

## Pre-computed Information

When a `ComputedGrid` is constructed, it spends extra time calculating a number of variables related to
the game grid. This information is documented in the [ComputedGrid documentation](https://rit-mdrc.github.io/mdrc-pacbot-util/api/mdrc_pacbot_util/grid/struct.ComputedGrid.html).