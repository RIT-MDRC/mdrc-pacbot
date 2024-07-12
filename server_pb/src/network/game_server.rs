use crate::{status, App};
use async_tungstenite::async_std::ConnectStream;
use async_tungstenite::WebSocketStream;
use core_pb::bin_encode;
use core_pb::messages::{GameServerCommand, NetworkStatus, GAME_SERVER_MAGIC_NUMBER};
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::threaded_websocket::{Address, TextOrT, ThreadedSocket};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::future::Either;
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};
use tokio::select;

pub async fn manage_game_server(
    app: Arc<Mutex<App>>,
    state_sender: UnboundedSender<GameState>,
    mut addrs: UnboundedReceiver<Option<Address>>,
    mut commands: UnboundedReceiver<GameServerCommand>,
) {
    let mut socket: ThreadedSocket<GameServerCommand, Vec<u8>> =
        ThreadedSocket::new::<WebSocketStream<ConnectStream>, _, _, _, _>(
            None,
            bin_encode,
            |bytes| Ok::<_, ()>(bytes.iter().copied().collect()),
        );

    loop {
        select! {
            addr = addrs.next() => {
                socket.connect(addr.expect("Game server address channel closed"));
            }
            command = commands.next() => {
                match command.expect("Game server command channel closed") {
                    GameServerCommand::Pause => {
                        socket.async_send(TextOrT::Text("p".into())).await;
                    }
                    GameServerCommand::Unpause => {
                        socket.async_send(TextOrT::Text("P".into())).await;
                    }
                    GameServerCommand::Reset => {
                        socket.async_send(TextOrT::Text("r".into())).await;
                    }
                    command => {
                        if app.lock().unwrap().status.advanced_game_server {
                            socket.async_send(TextOrT::T(command)).await;
                        }
                    }
                }
            }
            msg = socket.async_read() => {
                match msg {
                    Either::Left(TextOrT::Text(text)) => eprintln!("Unexpected text from game server: {text}"),
                    Either::Left(TextOrT::T(bytes)) => {
                        if bytes == GAME_SERVER_MAGIC_NUMBER.to_vec() {
                            status(&app, |s| s.advanced_game_server = true);
                        } else {
                        let mut g = GameState::new();
                            match g.update(&bytes) {
                                Ok(()) => state_sender.unbounded_send(g).unwrap(),
                                Err(e) => eprintln!("Error updating game state: {e:?}"),
                            }
                        }
                    }
                    Either::Right(new_status) => {
                        status(&app, |s| {
                            if new_status != NetworkStatus::Connected {
                                s.advanced_game_server = false;
                            }
                            s.game_server_connection_status = new_status
                        })
                    }
                }
            }
        }
    }
}
