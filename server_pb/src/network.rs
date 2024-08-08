use crate::sockets::Destination::*;
use crate::sockets::Incoming::*;
use crate::sockets::Outgoing::*;
use crate::sockets::{Destination, Incoming, Outgoing};
use crate::App;
use core_pb::messages::{
    GuiToServerMessage, NetworkStatus, RobotToServerMessage, ServerToGuiMessage,
    ServerToRobotMessage, GAME_SERVER_MAGIC_NUMBER,
};
use core_pb::pacbot_rs::game_state::GameState;

impl App {
    pub async fn handle_message(&mut self, from: Destination, message: Incoming) {
        match (from, message) {
            (dest, Bytes(data)) => eprintln!(
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
                Simulation => self.status.simulation_connection = status,
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
                                self.status.game_state = g;
                                self.status.rl_target = vec![];
                            }
                        }
                        Err(e) => eprintln!("Error updating game state: {e:?}"),
                    }
                }
            }
            (_, FromSimulation(msg)) => println!("Message from simulation: {msg:?}"),
            (Robot(name), FromRobot(RobotToServerMessage::Name(_))) => {
                println!("Received name from {name}");
                self.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::MotorConfig(
                        self.settings.robots[name as usize].motor_config,
                    )),
                )
                .await;
                self.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::Pid(
                        self.settings.robots[name as usize].pid,
                    )),
                )
                .await;
            }
            (Robot(name), FromRobot(RobotToServerMessage::MotorControlStatus(status))) => {
                self.status.robots[name as usize].last_motor_status = status;
            }
            (Robot(name), FromRobot(msg)) => println!("Message received from {name}: {msg:?}"),
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
                GuiToServerMessage::RobotVelocity(_robot, _vel) => {
                    // let (lin, ang) = vel.unwrap_or((Vector2::zeros(), 0.0));
                    // println!(
                    //     "sending vel {lin:?} {ang:?} = {:?} to robot..",
                    //     RobotDefinition::default()
                    //         .drive_system
                    //         .get_motor_speed_omni(lin, ang)
                    // );
                    // app.send(
                    //     Robot(robot),
                    //     ToRobot(ServerToRobotMessage::TargetVelocity(lin, ang)),
                    // )
                    // .await
                }
                _ => {}
            },
            (_, GuiConnected(id)) => {
                self.status.gui_clients += 1;
                println!(
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
                println!(
                    "Gui client #{id} disconnected; {} gui client(s) remaining",
                    self.status.gui_clients
                );
            }
            (dest, Incoming::Text(text)) => eprintln!("Unexpected text from {dest:?}: {text}"),
        }
    }
}
