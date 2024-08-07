use crate::high_level::ReinforcementLearningManager;
use crate::network::manage_network;
use crate::ota::OverTheAirProgramming;
use crate::sockets::{Destination, Outgoing, Sockets};
use core_pb::bin_encode;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::{ConnectionSettings, PacbotSettings};
use core_pb::messages::ServerToRobotMessage;
use core_pb::names::RobotName;
use core_pb::util::utilization::UtilizationMonitor;
use core_pb::util::StdInstant;
use std::process::{Child, Command};

mod high_level;
pub mod network;
mod ota;
mod sockets;
// todo pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,
    settings: PacbotSettings,
    utilization_monitor: UtilizationMonitor<100, StdInstant>,

    settings_update_needed: bool,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    sockets: Sockets,

    rl_manager: ReinforcementLearningManager,
    over_the_air_programming: OverTheAirProgramming,

    grid: ComputedGrid,
}

#[tokio::main]
async fn main() {
    println!("RIT Pacbot server starting up");

    manage_network().await;
}

impl App {
    async fn send(&mut self, destination: Destination, outgoing: Outgoing) {
        if self.settings.safe_mode {
            if let Outgoing::ToRobot(msg) = &outgoing {
                let encoded = bin_encode(msg.clone()).unwrap();
                if encoded[0] > 7 {
                    return;
                }
            }
        }
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
            let id = name as usize;
            self.update_connection(
                &old.robots[id].connection,
                &new.robots[id].connection,
                Destination::Robot(name),
            )
            .await;
            if old.robots[id].motor_config != new.robots[id].motor_config {
                self.send(
                    Destination::Robot(name),
                    Outgoing::ToRobot(ServerToRobotMessage::MotorConfig(
                        new.robots[id].motor_config,
                    )),
                )
                .await;
            }
            if old.robots[id].pid != new.robots[id].pid {
                self.send(
                    Destination::Robot(name),
                    Outgoing::ToRobot(ServerToRobotMessage::Pid(new.robots[id].pid)),
                )
                .await;
            }
        }

        if new.simulation.simulate {
            if self.sim_game_engine_process.is_none() {
                self.sim_game_engine_process = Some(
                    Command::new("cargo")
                        .args(["run", "--bin", "sim_pb", "--release"])
                        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../")
                        .spawn()
                        .unwrap(),
                );
            }
        } else if let Some(mut child) = self.sim_game_engine_process.take() {
            child.kill().unwrap();
        }

        if new.host_http {
            if self.client_http_host_process.is_none() {
                self.client_http_host_process = Some(
                    Command::new("trunk")
                        .args(["serve", "--config", "gui_pb/Trunk.toml"])
                        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../")
                        .spawn()
                        .unwrap(),
                );
            }
        } else if let Some(mut child) = self.client_http_host_process.take() {
            child.kill().unwrap();
        }

        self.settings = new;
        self.settings_update_needed = true;
    }
}
