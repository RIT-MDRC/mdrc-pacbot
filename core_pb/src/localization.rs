use crate::grid::standard_grid::StandardGrid;
use nalgebra::Point2;

pub fn estimate_location(
    _grid: StandardGrid,
    _cv_location: Option<Point2<i8>>,
    _distance_sensors: &[Result<Option<f32>, ()>; 4],
) -> Option<Point2<f32>> {
    None
}
