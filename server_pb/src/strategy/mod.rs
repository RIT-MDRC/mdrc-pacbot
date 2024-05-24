mod forward;
mod manual;
mod uniform;

use crate::strategy::manual::{ManualStrategy, StopStrategy};
use crate::strategy::uniform::UniformStrategy;
use crate::App;
use core_pb::messages::settings::StrategyChoice;
use nalgebra::{Point2, Rotation2, Vector2};

pub fn create_strategy(choice: &StrategyChoice) -> Box<dyn Strategy> {
    match choice {
        StrategyChoice::Stop => Box::new(StopStrategy::default()),
        StrategyChoice::Manual => Box::new(ManualStrategy::default()),
        StrategyChoice::ReinforcementLearning(_) => todo!(),
        StrategyChoice::TestUniform => Box::new(UniformStrategy::default()),
        StrategyChoice::TestForward => todo!(),
    }
}

/// The possible outputs for a Strategy, to be sent to a Navigator
///
/// Prefer more general output, in this order:
/// - Cell
/// - Location
/// - Path
/// - LinearVelocity
/// - Velocity
#[derive(Clone, Debug)]
pub enum StrategyResult {
    /// Preferred; an integer (row, col) grid cell coordinate; navigate via BFS to its center
    Cell(Point2<i8>),
    /// A floating point (row, col) coordinate; navigate via BFS, need not be in the center of a cell
    ///
    /// Prefer [`StrategyResult::Cell`] if using the center of the cell
    Location(Point2<f32>),
    /// A path of grid cells to follow when BFS is not sufficient
    ///
    /// Prefer [`StrategyResult::Cell`] if BFS navigation is acceptable
    Path(Vec<Point2<i8>>),
    /// A path of exact checkpoints to follow
    ///
    /// Prefer [`StrategyResult::Path`] if floating point coordinates are not needed
    Checkpoints(Vec<Point2<f32>>),
    /// Directly set the target velocity to these (row, col) speeds, in gu/s
    ///
    /// Rotational velocity may be adjusted to improve speed or pathing
    LinearVelocity(Vector2<f32>),
    /// Directly set the target velocity to these (row, col) speeds, in gu/s, and rotational
    /// speed, in rad/s
    ///
    /// Prefer [`StrategyResult::LinearVelocity`] if any rotational speed is acceptable
    Velocity(Vector2<f32>, Rotation2<f32>),
}

pub trait Strategy {
    /// Erase all contextual data so that the next decision is made as if the strategy was
    /// just created
    ///
    /// Will always be called after a grid change
    fn reset(&mut self, _app: &App) {}

    /// Run the strategy for the given state of the App
    fn run(&mut self, app: &App) -> StrategyResult;
}
