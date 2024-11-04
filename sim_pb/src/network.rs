use crate::driving::SimRobot;
use crate::{MyApp, RobotReference, Wall};
use bevy::prelude::{error, info, Commands, Entity, Query, ResMut, Resource, Transform};
use bevy_rapier2d::dynamics::{ExternalImpulse, Velocity};
use bevy_rapier2d::na::{Point2, Rotation2};
use core_pb::constants::{GAME_SERVER_MAGIC_NUMBER, GAME_SERVER_PORT, SIMULATION_LISTENER_PORT};
use core_pb::messages::{GameServerCommand, ServerToSimulationMessage, SimulationToServerMessage};
use core_pb::names::RobotName;
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::pacbot_rs::location::Direction::*;
use core_pb::{bin_decode_single, bin_encode};
use simple_websockets::{Event, EventHub, Message, Responder};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const GAME_FPS: f32 = 24.0;

#[derive(Resource)]
pub struct PacbotNetworkSimulation {
    pub game_state: GameState,
    pub last_state_update: Instant,

    pub event_hub: EventHub,
    pub game_server_clients: HashMap<u64, Responder>,

    pub simulation_event_hub: EventHub,
    pub simulation_clients: HashMap<u64, Responder>,
}

pub fn update_network(
    app: ResMut<MyApp>,
    mut network: ResMut<PacbotNetworkSimulation>,
    commands: Commands,
    walls: Query<(Entity, &Wall)>,
    robots: Query<(
        Entity,
        &mut Transform,
        &mut Velocity,
        &mut ExternalImpulse,
        &RobotReference,
    )>,
) {
    network.update(app, commands, walls, robots);
}

impl PacbotNetworkSimulation {
    pub fn new() -> Result<Self, simple_websockets::Error> {
        let event_hub = simple_websockets::launch(GAME_SERVER_PORT)?;
        info!("Listening on port {GAME_SERVER_PORT}");
        let simulation_event_hub = simple_websockets::launch(SIMULATION_LISTENER_PORT)?;
        let game_state = GameState {
            paused: true,
            ..Default::default()
        };
        Ok(Self {
            game_state,
            last_state_update: Instant::now(),

            event_hub,
            game_server_clients: HashMap::new(),

            simulation_event_hub,
            simulation_clients: HashMap::new(),
        })
    }

    /// All updates for network, game state, and simulation - will complete quickly, expects
    /// to be called in a loop
    pub fn update(
        &mut self,
        mut app: ResMut<MyApp>,
        mut commands: Commands,
        // mut pos_query: Query<&mut Transform>,
        walls: Query<(Entity, &Wall)>,
        mut robots: Query<(
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut ExternalImpulse,
            &RobotReference,
        )>,
    ) {
        while let Some(event) = self.event_hub.next_event() {
            match event {
                Event::Connect(id, responder) => {
                    info!("Client #{id} connected");
                    // this message lets clients know that this game server supports
                    // extra messages like pause, reset, custom game state
                    if !responder.send(Message::Binary(GAME_SERVER_MAGIC_NUMBER.to_vec())) {
                        error!("Error sending magic numbers, client already closed");
                    };
                    self.game_server_clients.insert(id, responder);
                    info!("{} client(s) connected", self.game_server_clients.len());
                }
                Event::Disconnect(id) => {
                    info!("Client #{id} disconnected");
                    self.game_server_clients.remove(&id);
                    info!("{} client(s) connected", self.game_server_clients.len());
                }
                Event::Message(id, msg) => match msg {
                    Message::Binary(bytes) => {
                        info!("Message received from rust client #{id}");
                        // binary messages originate from rust clients only
                        match bincode::serde::decode_from_slice::<GameServerCommand, _>(
                            &bytes,
                            bincode::config::standard(),
                        ) {
                            Ok((msg, _)) => match msg {
                                GameServerCommand::Pause => self.game_state.paused = true,
                                GameServerCommand::Unpause => self.game_state.paused = false,
                                GameServerCommand::Reset => self.game_state = GameState::default(),
                                GameServerCommand::Direction(dir) => {
                                    self.game_state.move_pacman_dir(dir)
                                }
                                GameServerCommand::SetState(s) => self.game_state = s,
                            },
                            Err(e) => {
                                error!("Couldn't deserialize client command from {:?}: {:?}", id, e)
                            }
                        }
                    }
                    Message::Text(ref s) => {
                        // text messages may originate from web clients
                        let chars = s.chars().collect::<Vec<_>>();
                        info!("Received message from {:?}: {:?}", id, msg.clone());
                        match chars[0] {
                            'p' => self.game_state.paused = true,
                            'P' => self.game_state.paused = false,
                            'r' | 'R' => self.game_state = GameState::default(),
                            'w' => self.game_state.move_pacman_dir(Up),
                            'a' => self.game_state.move_pacman_dir(Left),
                            's' => self.game_state.move_pacman_dir(Down),
                            'd' => self.game_state.move_pacman_dir(Right),
                            'x' => {
                                if s.len() != 3 {
                                    error!(
                                        "Received invalid position message from {:?}: '{:?}'",
                                        id, s
                                    )
                                } else {
                                    self.game_state
                                        .set_pacman_location((chars[1] as i8, chars[2] as i8));
                                }
                            }
                            _ => error!("Received unexpected message from {:?}: {:?}", id, s),
                        }
                    }
                },
            }
        }

        // simulation specific messages
        while let Some(event) = self.simulation_event_hub.next_event() {
            match event {
                Event::Connect(id, responder) => {
                    self.simulation_clients.insert(id, responder);
                }
                Event::Message(_, message) => match message {
                    Message::Binary(bytes) => {
                        match bin_decode_single::<ServerToSimulationMessage>(&bytes) {
                            Ok(msg) => match msg {
                                ServerToSimulationMessage::Spawn(name) => {
                                    app.spawn_robot(&mut commands, name);
                                }
                                ServerToSimulationMessage::Delete(name) => {
                                    app.despawn_robot(name, &mut commands);
                                }
                                ServerToSimulationMessage::SetPacman(name) => {
                                    app.selected_robot = name;
                                }
                                ServerToSimulationMessage::SetStandardGrid(grid) => {
                                    app.standard_grid = grid;
                                    app.grid = app.standard_grid.compute_grid();
                                    app.reset_grid(&walls, &mut robots, &mut commands)
                                }
                                ServerToSimulationMessage::Teleport(name, loc) => {
                                    if !app.grid.wall_at(&loc) {
                                        if let Some((_, mut transforms, ..)) = robots
                                            .iter_mut()
                                            .find(|(_, _, _, _, robot)| robot.0 == name)
                                        {
                                            transforms.translation.x = loc.x as f32;
                                            transforms.translation.y = loc.y as f32;
                                        }
                                    }
                                }
                                ServerToSimulationMessage::RobotButton(name, event) => {
                                    if let Some((_, sim_robot)) = &app.robots[name as usize] {
                                        sim_robot.write().unwrap().button_events.push_back(event)
                                    }
                                }
                                ServerToSimulationMessage::RobotJoystick(name, values) => {
                                    if let Some((_, sim_robot)) = &app.robots[name as usize] {
                                        sim_robot.write().unwrap().joystick = Some(values)
                                    }
                                }
                            },
                            Err(e) => error!("Error decoding simulation message: {e:?}"),
                        }
                    }
                    Message::Text(text) => error!("Unexpected simulation message: {text}"),
                },
                Event::Disconnect(id) => {
                    self.simulation_clients.remove(&id);
                }
            }
        }

        // send status to simulation clients
        for client in self.simulation_clients.values_mut() {
            client.send(Message::Binary(
                bin_encode(SimulationToServerMessage::RobotPositions(
                    RobotName::get_all().map(|name| {
                        if !name.is_simulated() {
                            None
                        } else {
                            app.robots[name as usize]
                                .iter()
                                .next()
                                .and_then(|(_, _)| {
                                    robots
                                        .iter_mut()
                                        .find(|(_, _, _, _, robot)| robot.0 == name)
                                })
                                .map(|(_, t, ..)| t)
                                .map(|t| {
                                    (
                                        Point2::new(t.translation.x, t.translation.y),
                                        // feels weird, but this does work
                                        Rotation2::new(
                                            2.0 * t.rotation.normalize().w.acos()
                                                * t.rotation.z.signum(),
                                        ),
                                    )
                                })
                        }
                    }),
                ))
                .unwrap(),
            ));
        }

        // send updated displays to clients
        for name in RobotName::get_all() {
            if !name.is_simulated() {
                continue;
            }
            if let Some((_, robot)) = &app.robots[name as usize] {
                let mut updated_display = None;
                {
                    let mut sim_robot = robot.write().unwrap();
                    if sim_robot.display_updated {
                        updated_display = Some(sim_robot.display.pixels);
                        sim_robot.display_updated = false;
                    }
                }
                if let Some(updated_display) = updated_display {
                    let updated_display: Vec<u128> = updated_display
                        .into_iter()
                        .map(|row| {
                            row.into_iter()
                                .enumerate()
                                .fold(0u128, |acc, (i, x)| acc | (u128::from(x) << i))
                        })
                        .collect();
                    for client in self.simulation_clients.values_mut() {
                        client.send(Message::Binary(
                            bin_encode(SimulationToServerMessage::RobotDisplay(
                                name,
                                updated_display.clone(),
                            ))
                            .unwrap(),
                        ));
                    }
                }
            }
        }

        // robot messages
        for (_, robot) in app.robots.iter_mut().flatten() {
            if let Some((name, swapped)) = {
                let mut sim_robot = robot.write().unwrap();
                if sim_robot.reboot {
                    let (name, swapped) = (sim_robot.name, sim_robot.firmware_updated);
                    sim_robot.destroy();
                    Some((name, swapped))
                } else {
                    None
                }
            } {
                *robot = SimRobot::start(name, swapped);
            }
        }

        // update the game state if it has been long enough
        if self.time_to_update().is_none() {
            if !self.game_state.paused {
                self.game_state.step();
            }
            // send game state to clients
            let serialized_state = self.game_state.to_bytes();
            for (id, responder) in &mut self.game_server_clients {
                if !responder.send(Message::Binary(serialized_state.clone())) {
                    error!("Failed to send game state to {id}: already closed");
                }
            }
            self.last_state_update = Instant::now();
        }
    }

    pub fn time_to_update(&self) -> Option<Duration> {
        let elapsed = self.last_state_update.elapsed();
        let interval = Duration::from_secs_f32(1.0 / GAME_FPS);
        if elapsed > interval {
            None
        } else {
            Some(interval - elapsed)
        }
    }
}
