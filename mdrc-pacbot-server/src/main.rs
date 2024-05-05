use crate::grid::ComputedGrid;
use crate::messages::settings::PacbotSettings;
use crate::network::{reconnect_sockets, Sockets};
use nalgebra::{Isometry2, Point2, Rotation2, Vector2};
use pacbot_rs::game_state::GameState;

pub mod grid;
pub mod messages;
mod navigation;
pub mod network;
pub mod strategy;

#[derive(Default)]
#[allow(dead_code)]
pub struct App {
    sockets: Sockets,

    grid: ComputedGrid,
    game: GameState,

    location: Isometry2<f32>,
    // guaranteed to be a walkable cell
    int_location: Point2<i8>,

    settings: PacbotSettings,
    wasd_qe_input: (Vector2<f32>, Rotation2<f32>),
}

fn main() {
    println!("Hello, world!");

    let mut app = App::default();

    reconnect_sockets(&mut app);
}
