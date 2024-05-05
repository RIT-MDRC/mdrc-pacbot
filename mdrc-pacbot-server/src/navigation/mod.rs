use crate::strategy::StrategyResult;
use crate::App;
use nalgebra::{Point2, Rotation2, Vector2};

pub enum NavigatorChoice {}

/// Convert a [`StrategyResult`] into a target velocity and rotational target velocity
pub fn navigate(
    app: &mut App,
    target: StrategyResult,
    navigator: Option<&mut dyn Navigator>,
) -> (Vector2<f32>, Rotation2<f32>) {
    let mut target = target;
    loop {
        match target {
            StrategyResult::Velocity(v, r) => return (v, r),
            _ => {
                target = match navigator {
                    Some(navigator) => navigator
                        .run(app, &target)
                        .unwrap_or_else(|| default_navigator(app, &target)),
                    None => default_navigator(app, &target),
                };
                todo!("record intermediate steps")
            }
        }
    }
}

pub trait Navigator {
    /// Erase all contextual data so that the next decision is made as if the navigator was
    /// just created
    fn reset(&mut self, _app: &App) {}

    /// Using the state of the app and the selected target, make the target more specific.
    ///
    /// This function will be repeatedly called with the [`StrategyResult`] it returns until
    /// it returns a [`StrategyResult::Velocity`]. Intermediary return values will be saved and
    /// sent to clients, for example so that a target path can be displayed on the GUI.
    ///
    /// If a [`Navigator`] should not exhibit special behavior for a given target, return
    /// [`None`] and a default navigator will be used.
    ///
    /// This method will never be called with a [`StrategyResult::Velocity`].
    fn run(&mut self, app: &App, target: &StrategyResult) -> Option<StrategyResult>;
}

/// The [`Navigator`] that is used when [`Navigator::run`] returns [`None`].
///
/// This function will be repeatedly called with the [`StrategyResult`] it returns until
/// it returns a [`StrategyResult::Velocity`]. Intermediary return values will be saved and
/// sent to clients, for example so that a target path can be displayed on the GUI.
///
/// This method should never be called with a [`StrategyResult::Velocity`].
fn default_navigator(app: &App, target: &StrategyResult) -> StrategyResult {
    match target {
        StrategyResult::Cell(c) => StrategyResult::Location(Point2::new(c.x as f32, c.y as f32)),
        StrategyResult::Location(p) => {
            if let Some(start) = app
                .grid
                .node_nearest(app.location.translation.x, app.location.translation.y)
            {
                if let Some(end) = app.grid.node_nearest(p.x, p.y) {
                    if let Some(path) = app.grid.bfs_path(start, end) {
                        let mut path: Vec<_> = path
                            .into_iter()
                            .map(|p| Point2::new(p.x as f32, p.y as f32))
                            .collect();
                        path.pop();
                        path.push(*p);
                        return StrategyResult::Checkpoints(path);
                    }
                }
            }
            // Invalid start or end position,
            StrategyResult::LinearVelocity(Vector2::new(0.0, 0.0))
        } // BFS
        StrategyResult::Path(p) => StrategyResult::Checkpoints(
            p.iter()
                .map(|c| Point2::new(c.x as f32, c.y as f32))
                .collect(),
        ),
        StrategyResult::Checkpoints(_) => todo!("pathing from old code"),
        StrategyResult::LinearVelocity(v) => StrategyResult::Velocity(*v, Rotation2::identity()),
        StrategyResult::Velocity(_, _) => {
            unreachable!("default_navigator was called with a StrategyResult::Velocity!")
        }
    }
}
