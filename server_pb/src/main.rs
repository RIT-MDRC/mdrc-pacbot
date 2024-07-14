use std::process::{Child, Command};

use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::{ConnectionSettings, PacbotSettings};
use core_pb::names::RobotName;

use crate::network::manage_network;
use crate::sockets::{Destination, Outgoing, Sockets};

pub mod network;
mod sockets;
// todo pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,
    settings: PacbotSettings,

    settings_update_needed: bool,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    sockets: Sockets,

    grid: ComputedGrid,
}

#[tokio::main]
async fn main() {
    println!("RIT Pacbot server starting up");

    manage_network().await;
}

impl App {
    async fn send(&mut self, destination: Destination, outgoing: Outgoing) {
        self.sockets
            .outgoing
            .send((destination, outgoing))
            .await
            .unwrap();
    }

    async fn update_connection(
        &mut self,
        old_settings: &ConnectionSettings,
        new_settings: &ConnectionSettings,
        destination: Destination,
    ) {
        if new_settings != old_settings {
            if new_settings.connect {
                self.send(
                    destination,
                    Outgoing::Address(Some((new_settings.ipv4, new_settings.port))),
                )
                .await;
            } else {
                self.send(destination, Outgoing::Address(None)).await;
            }
        }
    }

    async fn update_settings(&mut self, old: &PacbotSettings, new: PacbotSettings) {
        self.update_connection(
            &old.game_server.connection,
            &new.game_server.connection,
            Destination::GameServer,
        )
        .await;
        self.update_connection(
            &old.simulation.connection,
            &new.simulation.connection,
            Destination::Simulation,
        )
        .await;
        self.update_connection(
            &old.game_server.connection,
            &new.game_server.connection,
            Destination::GameServer,
        )
        .await;

        for name in RobotName::get_all() {
            self.update_connection(
                &old.robots[name as usize].connection,
                &new.robots[name as usize].connection,
                Destination::Robot(name),
            )
            .await;
        }

        if new.simulation.simulate {
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
