use crate::sockets::Destination::*;
use crate::sockets::Incoming::*;
use crate::sockets::Outgoing::*;
use crate::sockets::{Destination, Incoming, Outgoing};
use crate::App;
use core_pb::constants::GAME_SERVER_MAGIC_NUMBER;
use core_pb::messages::{
    GuiToServerMessage, NetworkStatus, RobotToServerMessage, ServerToGuiMessage,
    ServerToSimulationMessage, SimulationToServerMessage,
};
use core_pb::names::RobotName;
use core_pb::pacbot_rs::game_state::GameState;
use log::{error, info};
use nalgebra::Point2;

impl App {
    pub async fn handle_message(&mut self, from: Destination, message: Incoming) {
        match (from, message) {
            (dest, Bytes(data)) => error!(
                "Unexpectedly received {} raw bytes from {dest:?}",
                data.len()
            ),
            (_, SleepFinished) => {
                // send updated status to clients every so often
                self.send(
                    GuiClients,
                    ToGui(ServerToGuiMessage::Status(self.status.clone())),
                )
                .await
            }
            (dest, Status(status)) => match dest {
                Simulation => {
                    self.status.simulation_connection = status;
                    if status == NetworkStatus::Connected {
                        self.send(
                            Simulation,
                            ToSimulation(ServerToSimulationMessage::SetPacman(
                                self.settings.pacman,
                            )),
                        )
                        .await;
                        self.send(
                            Simulation,
                            ToSimulation(ServerToSimulationMessage::SetStandardGrid(
                                self.settings.standard_grid,
                            )),
                        )
                        .await;
                    }
                }
                Robot(name) => self.status.robots[name as usize].connection = status,
                GameServer => {
                    if status != NetworkStatus::Connected {
                        // assume the game server is not advanced until proven otherwise
                        self.status.advanced_game_server = false;
                    }
                    self.status.game_server_connection = status
                }
                _ => {}
            },
            (_, FromGameServer(bytes)) => {
                if bytes == GAME_SERVER_MAGIC_NUMBER.to_vec() {
                    self.status.advanced_game_server = true;
                } else {
                    match GameState::from_bytes(&bytes, self.status.game_state.seed) {
                        Ok(g) => {
                            if g != self.status.game_state {
                                let mut truncate_from = None;
                                for (i, loc) in self.status.target_path.iter().enumerate().rev() {
                                    if (loc.x, loc.y) == (g.pacman_loc.row, g.pacman_loc.col) {
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
                                self.status.game_state = g.clone();
                                if let Some(first) = self.status.target_path.first() {
                                    if (first.x - self.status.game_state.pacman_loc.row).abs()
                                        + (first.y - self.status.game_state.pacman_loc.col).abs()
                                        > 1
                                    {
                                        self.status.target_path.clear();
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Error updating game state: {e:?}"),
                    }
                }
            }
            (_, FromSimulation(msg)) => match msg {
                SimulationToServerMessage::RobotPositions(robot_positions) => {
                    for name in RobotName::get_all() {
                        self.status.robots[name as usize].sim_position =
                            robot_positions[name as usize];
                    }
                }
                SimulationToServerMessage::RobotDisplay(name, display) => {
                    self.status.robots[name as usize].display = Some(display);
                }
            },
            (Robot(name), FromRobot(RobotToServerMessage::Name(said_name))) => {
                info!("Received name ({said_name}) from {name}");
                if said_name != name {
                    error!("WARNING: Robot is having an identity crisis");
                }
                // the robot will receive motor and pid configuration via periodic actions
            }
            (Robot(name), FromRobot(RobotToServerMessage::MotorControlStatus(status))) => {
                self.status.robots[name as usize].last_motor_status = status;
            }
            (Robot(name), FromRobot(RobotToServerMessage::Sensors(sensors))) => {
                self.status.robots[name as usize].imu_angle = sensors.angle;
                self.status.robots[name as usize].distance_sensors = sensors.distances;
                self.status.robots[name as usize].estimated_location = sensors.location;
                self.status.robots[name as usize].battery = sensors.battery;
            }
            (Robot(name), FromRobot(RobotToServerMessage::Pong)) => {
                if let Some(t) = self.robot_ping_timers[name as usize] {
                    self.status.robots[name as usize].ping = Some(t.elapsed())
                }
            }
            (Robot(name), FromRobot(msg)) => info!("Message received from {name}: {msg:?}"),
            (Robot(_), _) => {}
            (_, FromRobot(_)) => {}
            (_, FromGui(msg)) => match msg {
                GuiToServerMessage::Settings(settings) => {
                    let old_settings = self.settings.clone();
                    self.update_settings(&old_settings, settings).await;
                }
                GuiToServerMessage::GameServerCommand(command) => match command.text() {
                    Some(text) => self.send(GameServer, Outgoing::Text(text.into())).await,
                    None => {
                        if self.status.advanced_game_server {
                            self.send(GameServer, ToGameServer(command)).await;
                        }
                    }
                },
                GuiToServerMessage::RobotVelocity(robot, vel) => {
                    self.settings.robots[robot as usize].config.target_velocity = vel;
                }
                GuiToServerMessage::TargetLocation(loc) => {
                    if !self.grid.wall_at(&loc) {
                        if let Some(path) = self.grid.bfs_path(
                            Point2::new(
                                self.status.game_state.pacman_loc.row,
                                self.status.game_state.pacman_loc.col,
                            ),
                            loc,
                        ) {
                            self.status.target_path = path.into_iter().skip(1).collect();
                        }
                    }
                }
                GuiToServerMessage::SimulationCommand(msg) => {
                    self.send(Simulation, ToSimulation(msg)).await;
                }
                GuiToServerMessage::RestartSimulation => {
                    if self.settings.simulation.simulate {
                        let old_settings = self.settings.clone();
                        let mut new_settings = old_settings.clone();
                        new_settings.simulation.simulate = false;
                        self.update_settings(&old_settings, new_settings).await;
                        let new_settings = old_settings.clone();
                        self.update_settings(&old_settings, new_settings).await;
                    }
                }
                _ => {}
            },
            (_, GuiConnected(id)) => {
                self.status.gui_clients += 1;
                info!(
                    "Gui client #{id} connected; {} gui client(s) are connected",
                    self.status.gui_clients
                );
                self.send(
                    GuiClients,
                    ToGui(ServerToGuiMessage::Settings(self.settings.clone())),
                )
                .await;
            }
            (_, GuiDisconnected(id)) => {
                self.status.gui_clients -= 1;
                info!(
                    "Gui client #{id} disconnected; {} gui client(s) remaining",
                    self.status.gui_clients
                );
            }
            (dest, Incoming::Text(text)) => error!("Unexpected text from {dest:?}: {text}"),
        }
    }
}
