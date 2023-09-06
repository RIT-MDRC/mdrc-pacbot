fn main() {
    use mdrc_pacbot_util::grid::ComputedGrid;
    use mdrc_pacbot_util::standard_grids::GRID_PLAYGROUND;
    let grid = ComputedGrid::try_from(GRID_PLAYGROUND).unwrap();
    let walls = grid.walls();
    for wall in walls {
        println!(
            "{}, {}, {}, {}",
            wall.left_bottom.x, wall.left_bottom.y, wall.right_top.x, wall.right_top.y
        );
    }
}
