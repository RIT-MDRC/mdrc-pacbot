use crate::high_level::ReinforcementLearningManager;
use crate::ota::OverTheAirProgramming;
use crate::sockets::Destination::{GuiClients, Simulation};
use crate::sockets::Incoming::FromRobot;
use crate::sockets::Outgoing::{ToGameServer, ToGui, ToSimulation};
use crate::sockets::{Destination, Outgoing, Sockets};
use crate::Destination::Robot;
use crate::Outgoing::ToRobot;
use core_pb::bin_encode;
use core_pb::constants::{GUI_LISTENER_PORT, MAX_ROBOT_PATH_LENGTH};
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::grid::standard_grid::StandardGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::{ConnectionSettings, PacbotSettings, StrategyChoice};
use core_pb::messages::{
    GameServerCommand, NetworkStatus, ServerToGuiMessage, ServerToRobotMessage,
    ServerToSimulationMessage,
};
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::pacbot_rs::location::Direction;
use core_pb::util::stopwatch::Stopwatch;
use core_pb::util::utilization::UtilizationMonitor;
use core_pb::util::StdInstant;
use env_logger::Builder;
use log::{info, LevelFilter};
use nalgebra::Point2;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::select;
use tokio::time::{interval, Instant, Interval};

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
    inference_timer: Stopwatch<1, 10, StdInstant>,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    sockets: Sockets,
    robot_ping_timers: [Option<Instant>; NUM_ROBOT_NAMES],

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
            inference_timer: Stopwatch::new(
                "Inference",
                Duration::from_secs_f32(0.5),
                Duration::from_secs_f32(1.0),
                100.0,
                100.0,
            ),

            client_http_host_process: None,
            sim_game_engine_process: None,

            rl_manager: ReinforcementLearningManager::default(),
            over_the_air_programming: OverTheAirProgramming::new(sockets.outgoing.clone()),

            sockets,
            robot_ping_timers: [None; 5],

            grid: Default::default(),
        }
    }
}

#[tokio::main]
async fn main() {
    Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .filter(Some("core_pb::threaded_websocket"), LevelFilter::Off) // silence threaded_websocket
        .init();

    info!("RIT Pacbot server starting up");

    let mut app = App::default();
    info!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");
    app.utilization_monitor.start();

    // apply default settings
    app.update_settings(&PacbotSettings::default(), PacbotSettings::default())
        .await;

    app.run_forever().await;
}

impl App {
    async fn run_forever(&mut self) {
        let mut periodic_interval = interval(Duration::from_millis(100));
        let mut move_interval = interval(Duration::from_secs_f32(1.0 / self.settings.target_speed));
        let mut previous_settings = self.settings.clone();

        loop {
            select! {
                _ = periodic_interval.tick() => {
                    self.utilization_monitor.start();
                    self.periodic_actions(&mut previous_settings, &mut move_interval).await;
                    self.utilization_monitor.stop();
                }
                _ = move_interval.tick() => {
                    self.utilization_monitor.start();
                    self.move_pacman().await;
                    self.utilization_monitor.stop();
                }
                msg = self.sockets.incoming.recv() => {
                    let msg = msg.unwrap();
                    // we want to measure the amount of time the server spends processing messages,
                    // which shouldn't include the amount of time spent waiting for messages
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

                    self.utilization_monitor.stop();
                    self.status.utilization = self.utilization_monitor.status();
                }
            }
        }
    }

    async fn move_pacman(&mut self) {
        // if the current pacman robot isn't connected, update game state with target path
        if let Some(target) = self.status.target_path.first() {
            if self.settings.do_target_path
                && self.status.advanced_game_server
                && self.status.robots[self.settings.pacman as usize].connection
                    == NetworkStatus::NotConnected
            {
                let dir = match (
                    target.x - self.status.game_state.pacman_loc.row,
                    target.y - self.status.game_state.pacman_loc.col,
                ) {
                    (-1, 0) => Direction::Up,
                    (1, 0) => Direction::Down,
                    (0, -1) => Direction::Left,
                    (0, 1) => Direction::Right,
                    _ => Direction::Stay,
                };
                if dir != Direction::Stay {
                    self.send(
                        Destination::GameServer,
                        ToGameServer(GameServerCommand::Direction(dir)),
                    )
                    .await;
                }
            }
        }
    }

    async fn periodic_actions(
        &mut self,
        previous_settings: &mut PacbotSettings,
        move_pacman_interval: &mut Interval,
    ) {
        // trigger pings to any robots who don't have an active ping
        for name in RobotName::get_all() {
            if self.status.robots[name as usize].connection == NetworkStatus::Connected
                && self.robot_ping_timers[name as usize]
                    .map(|x| x.elapsed().as_millis() > 500)
                    .unwrap_or(true)
            {
                self.robot_ping_timers[name as usize] = Some(Instant::now());
                self.send(Robot(name), ToRobot(ServerToRobotMessage::Ping))
                    .await;
            }
        }
        self.over_the_air_programming.tick(&mut self.status).await;
        if self.settings != *previous_settings {
            *previous_settings = self.settings.clone();
            *move_pacman_interval =
                interval(Duration::from_secs_f32(1.0 / self.settings.target_speed));
            self.send(
                GuiClients,
                ToGui(ServerToGuiMessage::Settings(self.settings.clone())),
            )
            .await; // check if new AI calculation is needed
        }
        // todo this should happen when the game state changes, not when one step has been taken
        if self.status.target_path.len() < 4
            && self.settings.driving.strategy == StrategyChoice::ReinforcementLearning
        {
            self.inference_timer.start();
            let mut rl_direction = self
                .rl_manager
                .hybrid_strategy(self.status.game_state.clone());
            let mut rl_vec = rl_direction.vector();
            self.status.target_path = vec![Point2::new(
                self.status.game_state.pacman_loc.row + rl_vec.0,
                self.status.game_state.pacman_loc.col + rl_vec.1,
            )];

            let mut future = self.status.game_state.clone();
            let mut i = 0;
            while i < 4 {
                future.set_pacman_location((
                    future.pacman_loc.row + rl_vec.0,
                    future.pacman_loc.col + rl_vec.1,
                ));
                rl_direction = self.rl_manager.hybrid_strategy(future.clone());
                rl_vec = rl_direction.vector();
                i += 1;
                self.status.target_path.push(Point2::new(
                    future.pacman_loc.row + rl_vec.0,
                    future.pacman_loc.col + rl_vec.1,
                ));
            }
            self.inference_timer.mark_completed("inference").unwrap();
            self.status.inference_time = self.inference_timer.status();
        }
        // send motor commands to robots
        for name in RobotName::get_all() {
            let mut data = self.settings.robots[name as usize].config.clone();
            if name == self.settings.pacman && self.settings.standard_grid == StandardGrid::Pacman {
                data.cv_location = Some(Point2::new(
                    self.status.game_state.pacman_loc.get_coords().0,
                    self.status.game_state.pacman_loc.get_coords().1,
                ));
                data.target_path = self
                    .status
                    .target_path
                    .clone()
                    .into_iter()
                    .take(MAX_ROBOT_PATH_LENGTH)
                    .collect();
                data.follow_target_path = self.settings.do_target_path;
            }
            self.send(
                Robot(name),
                ToRobot(ServerToRobotMessage::FrequentRobotItems(data)),
            )
            .await;
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
                Robot(name),
            )
            .await;
        }

        if new.standard_grid != old.standard_grid {
            self.send(
                Simulation,
                ToSimulation(ServerToSimulationMessage::SetStandardGrid(
                    new.standard_grid,
                )),
            )
            .await;
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
                        .args(["serve", "--release", "--config", "gui_pb/Trunk.toml"])
                        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../")
                        .spawn()
                        .unwrap(),
                );
            }
        } else if let Some(mut child) = self.client_http_host_process.take() {
            child.kill().unwrap();
        }

        if old.pacman != new.pacman {
            self.send(
                Simulation,
                ToSimulation(ServerToSimulationMessage::SetPacman(new.pacman)),
            )
            .await;
        }

        self.settings = new;
    }
}
