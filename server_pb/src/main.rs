use crate::high_level::ReinforcementLearningManager;
use crate::ota::OverTheAirProgramming;
use crate::sockets::Destination::GuiClients;
use crate::sockets::Incoming::FromRobot;
use crate::sockets::Outgoing::ToGui;
use crate::sockets::{Destination, Outgoing, Sockets};
use crate::Destination::Robot;
use crate::Outgoing::ToRobot;
use core_pb::bin_encode;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::{ConnectionSettings, PacbotSettings};
use core_pb::messages::{ServerToGuiMessage, ServerToRobotMessage};
use core_pb::names::RobotName;
use core_pb::util::utilization::UtilizationMonitor;
use core_pb::util::StdInstant;
use nalgebra::Point2;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

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

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    sockets: Sockets,

    rl_manager: ReinforcementLearningManager,
    over_the_air_programming: OverTheAirProgramming,

    grid: ComputedGrid,
}

impl Default for App {
    fn default() -> Self {
        let sockets = Sockets::spawn();

        App {
            status: Default::default(),
            settings: Default::default(),
            utilization_monitor: UtilizationMonitor::default(),

            client_http_host_process: None,
            sim_game_engine_process: None,

            rl_manager: ReinforcementLearningManager::default(),
            over_the_air_programming: OverTheAirProgramming::new(sockets.outgoing.clone()),

            sockets,

            grid: Default::default(),
        }
    }
}

#[tokio::main]
async fn main() {
    println!("RIT Pacbot server starting up");

    let mut app = App::default();
    println!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");
    app.utilization_monitor.start();

    // apply default settings
    app.update_settings(&PacbotSettings::default(), PacbotSettings::default())
        .await;

    app.run_forever().await;
}

impl App {
    async fn run_forever(&mut self) {
        let mut previous_200ms_tick = Instant::now();
        let mut previous_settings = self.settings.clone();

        loop {
            // frequently (but not too frequently) do some stuff
            if previous_200ms_tick.elapsed() > Duration::from_millis(200) {
                previous_200ms_tick = Instant::now();
                self.periodic_actions(&mut previous_settings).await;
            }

            self.over_the_air_programming.tick(&mut self.status).await;

            // we want to measure the amount of time the server spends processing messages,
            // which shouldn't include the amount of time spent waiting for messages
            self.utilization_monitor.stop();
            self.status.utilization = self.utilization_monitor.status();
            let msg = self.sockets.incoming.recv().await.unwrap();
            self.utilization_monitor.start();

            if self.settings.safe_mode {
                if let FromRobot(msg) = &msg.1 {
                    let encoded = bin_encode(msg.clone()).unwrap();
                    if encoded[0] > 7 {
                        continue;
                    }
                }
            }
            self.over_the_air_programming
                .update(&msg, &mut self.status)
                .await;
            self.handle_message(msg.0, msg.1).await;
        }
    }

    async fn periodic_actions(&mut self, previous_settings: &mut PacbotSettings) {
        if self.settings != *previous_settings {
            *previous_settings = self.settings.clone();
            self.send(
                GuiClients,
                ToGui(ServerToGuiMessage::Settings(self.settings.clone())),
            )
            .await; // check if new AI calculation is needed
        }
        if self.status.rl_target.is_empty() {
            let rl_direction = self
                .rl_manager
                .hybrid_strategy(self.status.game_state.clone());
            let rl_vec = rl_direction.vector();
            // todo multiple steps
            self.status.rl_target = vec![Point2::new(
                self.status.game_state.pacman_loc.row + rl_vec.0,
                self.status.game_state.pacman_loc.col + rl_vec.1,
            )];
        }
        // send motor commands to robots
        for name in RobotName::get_all() {
            let id = name as usize;
            // pwm overrides
            if self.settings.robots[id]
                .pwm_override
                .iter()
                .any(|x| x[0].is_some() || x[1].is_some())
            {
                self.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::PwmOverride(
                        self.settings.robots[id].pwm_override,
                    )),
                )
                .await;
            }
            // motor overrides
            if self.settings.robots[id]
                .set_point_override
                .iter()
                .any(|x| x.is_some())
            {
                self.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::MotorsOverride(
                        self.settings.robots[id].set_point_override,
                    )),
                )
                .await;
            }
        }
    }

    async fn send(&mut self, destination: Destination, outgoing: Outgoing) {
        if self.settings.safe_mode {
            if let ToRobot(msg) = &outgoing {
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
    }
}
