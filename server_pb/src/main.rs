use std::collections::HashMap;
use std::process::{Child, Command};
use std::time::Instant;

use simple_websockets::Responder;

use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;
use core_pb::messages::GameServerCommand;
use core_pb::threaded_websocket::ThreadedSocket;

use crate::network::manage_network;

pub mod network;
// todo pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,
    settings: PacbotSettings,

    last_status_update: Instant,
    settings_update_needed: bool,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    game_server_socket: ThreadedSocket<GameServerCommand, Vec<u8>>,

    gui_clients: HashMap<u64, Responder>,

    grid: ComputedGrid,
}

#[tokio::main]
async fn main() {
    println!("RIT Pacbot server starting up");

    manage_network().await;
}

impl App {
    async fn update_settings(&mut self, old: &PacbotSettings, new: PacbotSettings) {
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
                self.game_server_socket
                    .connect(Some((new.game_server.ipv4, new.game_server.ws_port)));
            } else {
                self.game_server_socket.connect(None);
            }
        }

        if new.simulate {
            if self.sim_game_engine_process.is_none() {
                self.sim_game_engine_process = Some(
                    Command::new("cargo")
                        .args(["run", "--bin", "sim_pb", "--release"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.sim_game_engine_process.take() {
                child.kill().unwrap();
            }
        }

        if new.host_http {
            if self.client_http_host_process.is_none() {
                self.client_http_host_process = Some(
                    Command::new("trunk")
                        .args(["serve", "--config", "gui_pb/Trunk.toml"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.client_http_host_process.take() {
                child.kill().unwrap();
            }
        }

        self.settings = new;
        self.settings_update_needed = true;
    }
}
