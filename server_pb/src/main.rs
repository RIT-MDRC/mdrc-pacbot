use crate::high_level::ReinforcementLearningManager;
use crate::logging::RobotLoggers;
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
use core_pb::grid::GRID_SIZE;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::{
    ConnectionSettings, CvLocationSource, PacbotSettings, ShouldDoTargetPath, StrategyChoice,
};
use core_pb::messages::{
    GameServerCommand, NetworkStatus, ServerToGuiMessage, ServerToRobotMessage,
    ServerToSimulationMessage,
};
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::pacbot_rs::location::Direction;
use core_pb::threaded_websocket::TextOrT;
use core_pb::util::stopwatch::Stopwatch;
use core_pb::util::utilization::UtilizationMonitor;
use core_pb::util::WebTimeInstant;
use env_logger::Builder;
use log::{info, warn, LevelFilter};
use nalgebra::Point2;
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use std::collections::HashSet;
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;
use tokio::select;
use tokio::time::{interval, Instant, Interval};

mod high_level;
mod logging;
pub mod network;
mod ota;
mod repeater;
mod sockets;
// todo pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,
    settings: PacbotSettings,
    utilization_monitor: UtilizationMonitor<100, WebTimeInstant>,
    inference_timer: Stopwatch<1, 10, WebTimeInstant>,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    sockets: Sockets,
    robot_ping_timers: [Option<Instant>; NUM_ROBOT_NAMES],
    robot_loggers: Option<RobotLoggers>,

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
            robot_ping_timers: [None; NUM_ROBOT_NAMES],
            robot_loggers: RobotLoggers::generate().ok(),

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

    let folder_path = Path::new("server_pb");
    if !folder_path.is_dir() {
        panic!("Please run the server from the repository root.");
    }

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
        let mut periodic_interval = interval(Duration::from_millis(20));
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
                            let encoded = bin_encode(false, TextOrT::T(msg.clone())).unwrap();
                            if encoded.get(9).map(|x| *x > 7).unwrap_or(false) {
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
            if (self.settings.do_target_path == ShouldDoTargetPath::Yes
                || self.settings.do_target_path == ShouldDoTargetPath::DoWhilePlayed
                    && !self.status.game_state.paused)
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
        // send motor commands to robots
        for name in RobotName::get_all() {
            let mut data = self.settings.robots[name as usize].config.clone();
            if name == self.settings.pacman {
                data.grid = self.settings.standard_grid;
                data.cv_location = self.status.cv_location;
                data.target_path = self
                    .status
                    .target_path
                    .clone()
                    .into_iter()
                    .take(MAX_ROBOT_PATH_LENGTH)
                    .collect();
                data.follow_target_path = self.settings.do_target_path == ShouldDoTargetPath::Yes
                    || self.settings.do_target_path == ShouldDoTargetPath::DoWhilePlayed
                        && !self.status.game_state.paused;
            }
            self.send(
                Robot(name),
                ToRobot(ServerToRobotMessage::FrequentRobotItems(data)),
            )
            .await;

            if self.settings.robots[name as usize].extra_opts_enabled {
                self.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::ExtraOpts(
                        self.settings.robots[name as usize].extra_opts,
                    )),
                )
                .await;
            }
        }
    }

    async fn send(&mut self, destination: Destination, outgoing: Outgoing) {
        if self.settings.safe_mode {
            if let ToRobot(msg) = &outgoing {
                let encoded = bin_encode(false, TextOrT::T(msg.clone())).unwrap();
                if encoded.get(9).map(|x| *x > 7).unwrap_or(false) {
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

    fn trigger_strategy_update(&mut self) {
        warn!("strategy update");
        const LOOKAHEAD_DIST: usize = 4;
        if let Some(cv_loc) = self.status.cv_location {
            match self.settings.driving.strategy {
                StrategyChoice::TestUniform => {
                    if self.status.target_path.is_empty() {
                        // find reachable location
                        if let Some(path) = self
                            .grid
                            .walkable_nodes()
                            .iter()
                            .filter(|x| **x != cv_loc)
                            .flat_map(|p| self.grid.bfs_path(cv_loc, *p))
                            .choose(&mut thread_rng())
                        {
                            self.status.target_path = path.into_iter().skip(1).collect();
                        }
                    }
                }
                StrategyChoice::ReinforcementLearning => {
                    self.inference_timer.start();
                    self.status.target_path.clear();

                    // if second AI
                    if !(self.status.game_state.pellet_at((3, 1))
                        || self.status.game_state.pellet_at((23, 1))
                        || self.status.game_state.pellet_at((3, 26))
                        || self.status.game_state.pellet_at((23, 26))
                        || self
                            .status
                            .game_state
                            .ghosts
                            .into_iter()
                            .any(|g| g.is_frightened()))
                    {
                        // and less than 10 pellets
                        if self.status.game_state.num_pellets <= 10 {
                            if let Some(end_path) =
                                self.find_game_ending_path(&self.status.game_state)
                            {
                                self.status.target_path = end_path;
                                return;
                            }
                        }
                    }

                    let mut future = self.status.game_state.clone();
                    while self.status.target_path.len() < LOOKAHEAD_DIST {
                        if self
                            .grid
                            .wall_at(&Point2::new(future.pacman_loc.row, future.pacman_loc.col))
                            || (((future.pacman_loc.row == 3) || (future.pacman_loc.row == 23))
                                && ((future.pacman_loc.col == 1) || (future.pacman_loc.col == 26))
                                && !self.status.target_path.is_empty())
                        {
                            break;
                        }
                        let rl_direction = self.rl_manager.hybrid_strategy(future.clone());
                        let rl_vec = rl_direction.vector();
                        let new_p = Point2::new(
                            future.pacman_loc.row + rl_vec.0,
                            future.pacman_loc.col + rl_vec.1,
                        );
                        if !self.status.target_path.contains(&new_p) && new_p != cv_loc {
                            self.status.target_path.push(new_p);
                        } else {
                            break;
                        }
                        future.set_pacman_location((
                            future.pacman_loc.row + rl_vec.0,
                            future.pacman_loc.col + rl_vec.1,
                        ));
                    }

                    self.inference_timer.mark_completed("inference").unwrap();
                    self.status.inference_time = self.inference_timer.status();
                }
                StrategyChoice::TestForward => {
                    while self.status.target_path.len() < LOOKAHEAD_DIST {
                        let last_loc = self.status.target_path.last().copied().unwrap_or(cv_loc);
                        if let Some(neighbor) = self
                            .grid
                            .neighbors(&last_loc)
                            .into_iter()
                            .filter(|x| !self.status.target_path.contains(x) && *x != cv_loc)
                            .choose(&mut thread_rng())
                        {
                            self.status.target_path.push(neighbor);
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            self.status.target_path.clear();
        }
    }

    fn find_game_ending_path(&self, game_state: &GameState) -> Option<Vec<Point2<i8>>> {
        let mut cur_pos = Point2::new(game_state.pacman_loc.row, game_state.pacman_loc.col);
        let mut path = Vec::new();

        let mut remaining_pellets = (0..GRID_SIZE)
            .flat_map(|row| (0..GRID_SIZE).map(move |col| Point2::new(row as i8, col as i8)))
            .filter(|&pos| game_state.pellet_at((pos.x, pos.y)))
            .collect::<HashSet<_>>();
        while let Some(&closest_pellet) = remaining_pellets
            .iter()
            .min_by_key(|&pellet_pos| self.grid.dist(&cur_pos, pellet_pos))
        {
            for path_pos in self.grid.bfs_path(cur_pos, closest_pellet)? {
                // If any ghosts are too close to this location (extrapolating ahead in time pessimistically),
                // then abort and return None.
                if game_state.ghosts.iter().any(|ghost| {
                    // check if too close
                    let ghost_pos = Point2::new(ghost.loc.row, ghost.loc.col);
                    if let Some(dist_from_ghost) =
                        Some((path_pos.x - ghost_pos.x).abs() + (path_pos.y - ghost_pos.y).abs())
                    {
                        let num_pacman_moves = path.len();
                        let num_ghost_moves = ((10.0 / game_state.update_period as f32) // todo add as a setting
                            * num_pacman_moves as f32)
                            + 2.0;
                        // println!(
                        //     "{num_pacman_moves} {dist_from_ghost} {num_ghost_moves} {ghost_pos:?} {:?} {:?} {:?}"
                        // , self.grid.dist(&path_pos, &ghost_pos), path_pos, ghost_pos);
                        (dist_from_ghost as f32) < num_ghost_moves
                    } else {
                        false // no path from ghost to pacman
                    }
                }) {
                    return None;
                }

                let is_start_location = path.is_empty() && path_pos == cur_pos;
                let is_last_path_pos = path.last().is_some_and(|&last| last == path_pos);
                if !is_start_location && !is_last_path_pos {
                    path.push(path_pos);
                }
            }

            if let Some(&last) = path.last() {
                cur_pos = last;
            }

            remaining_pellets.remove(&closest_pellet);
        }

        Some(path)
    }

    fn trigger_cv_location_update(&mut self) {
        let old_loc = self.status.cv_location;
        self.status.cv_location = match self.settings.cv_location_source {
            CvLocationSource::GameState => Some(Point2::new(
                self.status.game_state.pacman_loc.row,
                self.status.game_state.pacman_loc.col,
            )),
            CvLocationSource::Constant(p) => p,
            CvLocationSource::Localization => self.status.robots[self.settings.pacman as usize]
                .estimated_location
                .and_then(|p| self.grid.node_nearest(p.x, p.y))
                .or(old_loc),
        };

        if old_loc != self.status.cv_location {
            if let Some(cv_loc) = self.status.cv_location {
                let mut truncate_from = None;
                for (i, loc) in self.status.target_path.iter().enumerate().rev() {
                    if *loc == cv_loc {
                        truncate_from = Some(i + 1);
                        break;
                    }
                }
                if let Some(truncate_from) = truncate_from {
                    self.status.target_path = self
                        .status
                        .target_path
                        .clone()
                        .into_iter()
                        .skip(truncate_from)
                        .collect();
                }
                if let Some(first) = self.status.target_path.first() {
                    if (first.x - cv_loc.x).abs() + (first.y - cv_loc.y).abs() > 1 {
                        self.status.target_path.clear();
                    }
                }
            } else {
                self.status.target_path.clear();
            }
            self.trigger_strategy_update();
        }
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
            self.grid = new.standard_grid.compute_grid();
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
                        .args(["run", "--bin", "sim_pb"])
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

        if old.driving.strategy != new.driving.strategy || old.standard_grid != new.standard_grid {
            self.status.target_path.clear();
            self.settings.driving.strategy = new.driving.strategy.clone();
            self.trigger_strategy_update();
        }

        self.trigger_cv_location_update();

        self.settings = new;
    }
}
