use crate::logging::RobotLoggers;
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

impl App {
    pub async fn handle_message(&mut self, from: Destination, message: Incoming) {
        match (from, message) {
            (Robot(name), Bytes(data)) => {
                if let Some(loggers) = &mut self.robot_loggers {
                    loggers.feed_robot_logs(name, &data)
                }
            }
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
                                self.status.game_state = g.clone();
                                self.trigger_cv_location_update();
                                self.trigger_strategy_update();
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
            (Robot(name), FromRobot(RobotToServerMessage::Utilization(utilization))) => {
                self.status.robots[name as usize].utilization = utilization;
            }
            (Robot(name), FromRobot(RobotToServerMessage::Sensors(sensors))) => {
                self.status.robots[name as usize].imu_angle =
                    sensors.angle.map_err(|s| s.to_string());
                self.status.robots[name as usize].distance_sensors =
                    sensors.distances.map(|x| x.map_err(|s| s.to_string()));
                self.status.robots[name as usize].estimated_location = sensors.location;
                self.status.robots[name as usize].battery = sensors.battery;
                self.trigger_cv_location_update();
            }
            (Robot(name), FromRobot(RobotToServerMessage::Pong)) => {
                if let Some(t) = self.robot_ping_timers[name as usize] {
                    self.status.robots[name as usize].ping = Some(t.elapsed())
                }
            }
            (Robot(name), FromRobot(RobotToServerMessage::Rebooting)) => {
                info!("{name} rebooting");
                self.send(Robot(name), Address(None)).await;
                self.send(
                    Robot(name),
                    Address(Some((
                        self.settings.robots[name as usize].connection.ipv4,
                        self.settings.robots[name as usize].connection.port,
                    ))),
                )
                .await;
            }
            (Robot(name), FromRobot(RobotToServerMessage::ReceivedExtraOpts(opts))) => {
                self.status.robots[name as usize].received_extra_opts = Some(opts);
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
                        if let Some(cv_loc) = self.status.cv_location {
                            if let Some(path) = self.grid.bfs_path(cv_loc, loc) {
                                self.status.target_path = path.into_iter().skip(1).collect();
                            }
                        }
                    }
                }
                GuiToServerMessage::SimulationCommand(msg) => {
                    self.send(Simulation, ToSimulation(msg)).await;
                }
                GuiToServerMessage::RobotCommand(name, msg) => {
                    self.send(Robot(name), ToRobot(msg)).await;
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
                GuiToServerMessage::StartOtaFirmwareUpdate(_) => {
                    // the ELF file probably changed
                    self.robot_loggers = RobotLoggers::generate().ok();
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
