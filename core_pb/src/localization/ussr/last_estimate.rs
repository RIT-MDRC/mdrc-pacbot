#[derive(Debug)]
struct LastEstimate<'a> {
    distance_sensors: &'a [Option<Option<f32>>; 4],
    final_estimate: Option<Point2<f32>>,
    velocity: Option<Point2<f32>>,
    timestamp: SystemTime
}