use crate::grid::standard_grid::StandardGrid;
use nalgebra::Point2;

pub fn estimate_location(
    _grid: StandardGrid,
    _distance_sensors: &[Result<Option<f32>, ()>; 4],
) -> Option<Point2<f32>> {
    None
}
