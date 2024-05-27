use crate::strategy::{Strategy, StrategyResult};
use crate::App;
use nalgebra::Point2;
use rand::prelude::{IteratorRandom, SliceRandom};
use rand::thread_rng;

#[derive(Default)]
pub struct ForwardStrategy {
    path: Vec<Point2<i8>>,
}

impl Strategy for ForwardStrategy {
    fn reset(&mut self, _app: &App) {
        self.path = vec![];
    }

    fn run(&mut self, app: &App) -> StrategyResult {
        let mut rng = thread_rng();
        // are we there yet?
        if self.path.first() == Some(&app.pacbot_int_location) {
            self.path.remove(0);
        }
        // invalidate current path if necessary
        if let Some(first) = self.path.first() {
            if let Some(path) = app.grid.bfs_path(app.pacbot_int_location, *first) {
                if path.len() != 1 {
                    self.path = vec![];
                }
            } else {
                self.path = vec![];
            }
        }
        // find first cell in path
        if self.path.is_empty() {
            if let Some(neighbor) = app
                .grid
                .neighbors(&app.pacbot_int_location)
                .choose(&mut rng)
            {
                self.path.push(*neighbor)
            }
        }
        // find second cell in path
        if self.path.len() == 1 {
            if let Some(neighbor) = app
                .grid
                .neighbors(&self.path[0])
                .iter()
                .filter(|x| **x != app.pacbot_int_location)
                .choose(&mut rng)
            {
                self.path.push(*neighbor)
            }
        }
        // fill out the rest of the path
        if self.path.len() > 1 {
            while self.path.len() < 5 {
                if let Some(neighbor) = app
                    .grid
                    .neighbors(&self.path[self.path.len()])
                    .iter()
                    .filter(|x| **x != self.path[self.path.len() - 1])
                    .choose(&mut rng)
                {
                    self.path.push(*neighbor)
                }
            }
        }
        StrategyResult::Path(self.path.clone())
    }
}
