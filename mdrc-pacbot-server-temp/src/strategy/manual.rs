use crate::strategy::{Strategy, StrategyResult};
use crate::App;
use nalgebra::{Rotation2, Vector2};

#[derive(Default)]
pub struct StopStrategy;

impl Strategy for StopStrategy {
    fn run(&mut self, _app: &App) -> StrategyResult {
        StrategyResult::Velocity(Vector2::new(0.0, 0.0), Rotation2::identity())
    }
}

#[derive(Default)]
pub struct ManualStrategy;

impl Strategy for ManualStrategy {
    fn run(&mut self, app: &App) -> StrategyResult {
        StrategyResult::Velocity(app.wasd_qe_input.0, app.wasd_qe_input.1)
    }
}
