use crate::strategy::{Strategy, StrategyResult};
use crate::App;
use nalgebra::{Point2, Vector2};
use rand::prelude::SliceRandom;
use rand::thread_rng;

#[derive(Default)]
pub struct UniformStrategy {
    current_target: Option<Point2<i8>>,
}

impl Strategy for UniformStrategy {
    fn reset(&mut self, _app: &App) {
        self.current_target = None;
    }

    fn run(&mut self, app: &App) -> StrategyResult {
        if let Some(current_target) = self.current_target {
            // are we there yet?
            if current_target == app.int_location {
                self.current_target = None;
            }
        }
        // return or create new target
        if let Some(current_target) = self.current_target {
            StrategyResult::Cell(current_target)
        } else {
            // look for a new target
            let potential_targets: Vec<_> = app
                .grid
                .walkable_nodes()
                .iter()
                .filter(|p| app.grid.bfs_path(app.int_location, **p).is_some())
                .collect();
            if let Some(target) = potential_targets.choose(&mut thread_rng()) {
                self.current_target = Some(**target);
                StrategyResult::Cell(**target)
            } else {
                // no available targets
                StrategyResult::LinearVelocity(Vector2::new(0.0, 0.0))
            }
        }
    }
}
