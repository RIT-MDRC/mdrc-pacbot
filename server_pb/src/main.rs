use std::process::{Child, Command};

use nalgebra::{Isometry2, Point2, Rotation2, Vector2};

use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;
use core_pb::pacbot_rs::game_state::GameState;

mod navigation;
pub mod network;
pub mod strategy;

#[derive(Default)]
#[allow(dead_code)]
pub struct App {
    status: ServerStatus,

    sim_game_engine_thread: Option<Child>,

    grid: ComputedGrid,
    game: GameState,

    pacbot_location: Isometry2<f32>,
    // guaranteed to be a walkable cell
    pacbot_int_location: Point2<i8>,

    settings: PacbotSettings,
    wasd_qe_input: (Vector2<f32>, Rotation2<f32>),
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // let mut app = App::default();
    //
    // loop {
    //     reconnect_sockets(&mut app);
    //     sleep(Duration::from_millis(50));
    // }
}

impl App {
    fn update_settings(&mut self, old: &PacbotSettings, new: &PacbotSettings) {
        if (
            new.game_server.connect,
            new.game_server.ipv4,
            new.game_server.ws_port,
        ) != (
            old.game_server.connect,
            old.game_server.ipv4,
            old.game_server.ws_port,
        ) {
            if new.game_server.connect {
                // todo
            } else {
            }
        }

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
