#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::grid::Grid;
use crate::grid::GridValue::*;

pub const GRID_PACMAN: Grid = [
//  bottom left of pacman board                                           // top left of pacman board
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I], // 0
    [I, o, o, o, o, I, I, O, o, o, o, I, I, I, I, I, I, I, I, I, I, I, o, o, o, o, o, O, o, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I], // 5
    [I, o, I, I, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, e, e, e, e, e, e, e, e, e, I, I, o, o, o, o, I, e, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, I, I, I, I, e, I, I, o, I, I, o, I, e, I, o, I, I], // 10
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, n, n, n, I, e, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, o, o, o, I, I, o, o, o, o, I, I, e, I, n, n, n, I, e, e, e, o, I, I, o, o, o, o, o, I, I],
    [I, o, I, I, I, I, I, e, I, I, I, I, I, e, I, n, n, n, n, e, I, I, I, I, I, o, I, I, I, I, I, I],
    [I, o, I, I, I, I, I, e, I, I, I, I, I, e, I, n, n, n, n, e, I, I, I, I, I, o, I, I, I, I, I, I],
    [I, o, o, o, o, I, I, o, o, o, o, I, I, e, I, n, n, n, I, e, e, e, o, I, I, o, o, o, o, o, I, I], // 15
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, n, n, n, I, e, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, e, I, I, I, I, I, e, I, I, o, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, e, e, e, e, e, e, e, e, e, I, I, o, o, o, o, I, e, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, I, I, I, o, I, I, o, I, I, I, I, I, e, I, I, I, I, I, I, I, I, o, I, I, I, o, I, I], // 20
    [I, o, I, I, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I],
    [I, o, I, I, o, I, I, I, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, o, o, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, e, I, o, I, I],
    [I, o, I, I, o, I, I, o, I, I, o, I, I, I, I, I, I, I, I, I, I, I, o, I, I, o, I, I, I, o, I, I], // 25
    [I, o, o, o, o, I, I, O, o, o, o, I, I, I, I, I, I, I, I, I, I, I, o, o, o, o, o, O, o, o, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
//   |              |              |              |              |              |              |   top right of pacman board
//   0              5              10             15             20             25             30
];

pub const GRID_BLANK: Grid = [
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, o, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I],
    [I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I]
];