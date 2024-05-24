use crate::network::{reconnect_sockets, Sockets};
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::settings::PacbotSettings;
use core_pb::pacbot_rs::game_state::GameState;
use nalgebra::{Isometry2, Point2, Rotation2, Vector2};

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
