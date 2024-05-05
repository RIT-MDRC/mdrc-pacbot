use crate::messages::{GameServerCommand, GAME_SERVER_MAGIC_NUMBER};
use pacbot_rs::game_state::GameState;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use tungstenite::{accept, Message, WebSocket};

pub const GAME_SERVER_PORT: u16 = 3002;
pub const GAME_FPS: f32 = 24.0;

pub struct PacbotSimulation {
    game_state: GameState,
    game_state_paused: bool,
    last_state_update: Instant,

    game_server_listener: TcpListener,
    game_server_clients: Vec<(WebSocket<TcpStream>, SocketAddr)>,
}

impl PacbotSimulation {
    pub fn new() -> io::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{GAME_SERVER_PORT}"))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            game_state: GameState::new(),
            game_state_paused: true,
            last_state_update: Instant::now(),

            game_server_listener: listener,
            game_server_clients: vec![],
        })
    }

    /// All updates for network, game state, and simulation - will complete quickly, expects
    /// to be called in a loop
    pub fn update(&mut self) {
        // accept new game server connections
        loop {
            match self.game_server_listener.accept() {
                Ok((socket, addr)) => {
                    match accept(socket) {
                        Ok(mut ws) => {
                            // this message lets clients know that this game server supports
                            // extra messages like pause, reset, custom game state
                            if let Err(e) =
                                ws.send(Message::Binary(GAME_SERVER_MAGIC_NUMBER.to_vec()))
                            {
                                eprintln!("Error sending magic numbers: {:?}", e);
                            };
                            self.game_server_clients.push((ws, addr));
                        }
                        Err(e) => {
                            eprintln!(
                                "Error upgrading game server socket from {:?}: {:?}",
                                addr, e
                            );
                        }
                    }
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => break,
                    _ => {
                        eprintln!("Error accepting game server TCP socket: {:?}", e);
                    }
                },
            }
        }

        // update the game state if it has been long enough
        if self.time_to_update().is_none() {
            if !self.game_state_paused {
                self.game_state.step();
            }
            // send game state to clients
            let serialized_state =
                bincode::serde::encode_to_vec(&self.game_state, bincode::config::standard())
                    .expect("Failed to serialize game state!");
            for (client, addr) in &mut self.game_server_clients {
                if let Err(e) = client.send(Message::Binary(serialized_state.clone())) {
                    eprintln!("Failed to send game state to {:?}: {:?}", addr, e);
                }
            }
            self.last_state_update = Instant::now();
        }

        // handle commands from game server clients
        for (client, addr) in &mut self.game_server_clients {
            while let Ok(msg) = client.read() {
                match msg {
                    Message::Binary(bytes) => {
                        match bincode::serde::decode_from_slice::<GameServerCommand, _>(
                            &bytes,
                            bincode::config::standard(),
                        ) {
                            Ok((msg, _)) => match msg {
                                GameServerCommand::Pause => self.game_state_paused = true,
                                GameServerCommand::Unpause => self.game_state_paused = false,
                                GameServerCommand::Reset => self.game_state = GameState::new(),
                                GameServerCommand::SetState(s) => self.game_state = s,
                            },
                            Err(e) => eprintln!(
                                "Couldn't deserialize client command from {:?}: {:?}",
                                addr, e
                            ),
                        }
                    }
                    _ => eprintln!("Received unexpected message from {:?}: {:?}", addr, msg),
                }
            }
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
