use std::collections::HashMap;
use std::time::{Duration, Instant};

use bevy::prelude::{ResMut, Resource};
use simple_websockets::{Event, EventHub, Message, Responder};

use core_pb::constants::{GAME_SERVER_PORT, SIMULATION_LISTENER_PORT};
use core_pb::messages::{
    GameServerCommand, ServerToSimulationMessage, SimulationToServerMessage,
    GAME_SERVER_MAGIC_NUMBER,
};
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::pacbot_rs::location::{LocationState, DOWN, LEFT, RIGHT, UP};
use core_pb::{bin_decode, bin_encode};

use crate::driving::SimRobot;
use crate::{MyApp, RobotToSimulationMessage};

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

pub fn update_network(app: ResMut<MyApp>, mut network: ResMut<PacbotNetworkSimulation>) {
    network.update(app)
}

impl PacbotNetworkSimulation {
    pub fn new() -> Result<Self, simple_websockets::Error> {
        let event_hub = simple_websockets::launch(GAME_SERVER_PORT)?;
        println!("Listening on port {GAME_SERVER_PORT}");
        let simulation_event_hub = simple_websockets::launch(SIMULATION_LISTENER_PORT)?;
        let mut game_state = GameState::default();
        game_state.paused = true;
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
    pub fn update(&mut self, mut app: ResMut<MyApp>) {
        while let Some(event) = self.event_hub.next_event() {
            match event {
                Event::Connect(id, responder) => {
                    println!("Client #{id} connected");
                    // this message lets clients know that this game server supports
                    // extra messages like pause, reset, custom game state
                    if !responder.send(Message::Binary(GAME_SERVER_MAGIC_NUMBER.to_vec())) {
                        eprintln!("Error sending magic numbers, client already closed");
                    };
                    self.game_server_clients.insert(id, responder);
                    println!("{} client(s) connected", self.game_server_clients.len());
                }
                Event::Disconnect(id) => {
                    println!("Client #{id} disconnected");
                    self.game_server_clients.remove(&id);
                    println!("{} client(s) connected", self.game_server_clients.len());
                }
                Event::Message(id, msg) => match msg {
                    Message::Binary(bytes) => {
                        println!("Message received from rust client #{id}");
                        // binary messages originate from rust clients only
                        match bincode::serde::decode_from_slice::<GameServerCommand, _>(
                            &bytes,
                            bincode::config::standard(),
                        ) {
                            Ok((msg, _)) => match msg {
                                GameServerCommand::Pause => self.game_state.paused = true,
                                GameServerCommand::Unpause => self.game_state.paused = false,
                                GameServerCommand::Reset => self.game_state = GameState::default(),
                                GameServerCommand::SetState(s) => self.game_state = s,
                            },
                            Err(e) => eprintln!(
                                "Couldn't deserialize client command from {:?}: {:?}",
                                id, e
                            ),
                        }
                    }
                    Message::Text(ref s) => {
                        // text messages may originate from web clients
                        let chars = s.chars().collect::<Vec<_>>();
                        println!("Received message from {:?}: {:?}", id, msg.clone());
                        match chars[0] {
                            'p' => self.game_state.paused = true,
                            'P' => self.game_state.paused = false,
                            'r' | 'R' => self.game_state = GameState::default(),
                            'w' => self.game_state.move_pacman_dir(UP),
                            'a' => self.game_state.move_pacman_dir(LEFT),
                            's' => self.game_state.move_pacman_dir(DOWN),
                            'd' => self.game_state.move_pacman_dir(RIGHT),
                            'x' => {
                                if s.len() != 3 {
                                    eprintln!(
                                        "Received invalid position message from {:?}: '{:?}'",
                                        id, s
                                    )
                                } else {
                                    let new_loc = LocationState {
                                        row: chars[1] as i8,
                                        col: chars[2] as i8,
                                        dir: UP, // TODO this is not really correct
                                    };
                                    self.game_state.set_pacman_location(new_loc);
                                }
                            }
                            _ => eprintln!("Received unexpected message from {:?}: {:?}", id, s),
                        }
                    }
                },
            }
        }

        // simulation specific messages
        while let Some(event) = self.simulation_event_hub.next_event() {
            match event {
                Event::Connect(id, responder) => {
                    responder.send(Message::Binary(
                        bin_encode(SimulationToServerMessage::None).unwrap(),
                    ));
                    self.simulation_clients.insert(id, responder);
                }
                Event::Message(_, message) => match message {
                    Message::Binary(bytes) => match bin_decode::<ServerToSimulationMessage>(&bytes)
                    {
                        Ok(msg) => println!("got message: {msg:?}"),
                        Err(e) => eprintln!("Error decoding simulation message: {e:?}"),
                    },
                    Message::Text(text) => eprintln!("Unexpected simulation message: {text}"),
                },
                Event::Disconnect(id) => {
                    self.simulation_clients.remove(&id);
                }
            }
        }

        // robot messages
        while let Ok((name, msg)) = app.from_robots.1.try_recv() {
            match msg {
                RobotToSimulationMessage::SimulatedVelocity(lin, ang) => {
                    if Some((lin, ang)) != app.server_target_vel[name as usize] {
                        println!("Received target velocity: {lin:?} {ang:?}");
                        app.server_target_vel[name as usize] = Some((lin, ang))
                    }
                }
                RobotToSimulationMessage::MarkFirmwareUpdated => {
                    if let Some((_, sim_robot)) = &mut app.robots[name as usize] {
                        println!("{name} declared updated firmware");
                        sim_robot.write().unwrap().firmware_updated = true;
                    }
                }
                RobotToSimulationMessage::Reboot => {
                    let tx = app.from_robots.0.clone();
                    if let Some((_, sim_robot)) = &mut app.robots[name as usize] {
                        let swapped;
                        {
                            let mut r = sim_robot.write().unwrap();
                            swapped = r.firmware_updated;
                            r.destroy();
                        }
                        *sim_robot = SimRobot::start(name, swapped, tx);
                    }
                }
            }
        }

        // update the game state if it has been long enough
        if self.time_to_update().is_none() {
            if !self.game_state.paused {
                self.game_state.step();
            }
            // send game state to clients
            let serialized_state = self.game_state.get_bytes();
            for (id, responder) in &mut self.game_server_clients {
                if !responder.send(Message::Binary(serialized_state.clone())) {
                    eprintln!("Failed to send game state to {id}: already closed");
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
