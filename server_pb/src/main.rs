use crate::network::{reconnect_sockets, Sockets};
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::settings::PacbotSettings;
use core_pb::pacbot_rs::game_state::GameState;
use nalgebra::{Isometry2, Point2, Rotation2, Vector2};
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

mod navigation;
pub mod network;
pub mod strategy;

#[derive(Default)]
#[allow(dead_code)]
pub struct App {
    sockets: Sockets,
    sim_game_engine_thread: Option<Child>,

    grid: ComputedGrid,
    game: GameState,

    pacbot_location: Isometry2<f32>,
    // guaranteed to be a walkable cell
    pacbot_int_location: Point2<i8>,

    settings: PacbotSettings,
    wasd_qe_input: (Vector2<f32>, Rotation2<f32>),
}

fn main() {
    println!("Hello, world!");

    let mut app = App::default();

    loop {
        reconnect_sockets(&mut app);
        sleep(Duration::from_millis(50));
    }
}

impl App {
    fn update_settings(&mut self, _old: &PacbotSettings, new: &PacbotSettings) {
        if new.game_server.simulate {
            if self.sim_game_engine_thread.is_none() {
                self.sim_game_engine_thread = Some(
                    Command::new("cargo")
                        .args(["run", "--bin", "sim_pb"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.sim_game_engine_thread.take() {
                child.kill().unwrap();
            }
        }
    }
}
