use crate::{status, App};
use core_pb::messages::GameServerCommand;
use core_pb::pacbot_rs::game_state::GameState;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::future::{select, Either};
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub async fn manage_game_server(
    app: Arc<Mutex<App>>,
    state_sender: UnboundedSender<GameState>,
    mut commands: UnboundedReceiver<GameServerCommand>,
) {
    let mut s: Option<WebSocketStream<MaybeTlsStream<TcpStream>>> = None;
    let mut addr: Option<([u8; 4], u16)> = None;

    loop {
        if let Some(socket) = &mut s {
            match select(commands.next(), socket.next()).await {
                // success game server command
                Either::Left((Some(x), _)) => match x {
                    GameServerCommand::Connect(a) => {
                        addr = a;
                        s = None;
                    }
                    GameServerCommand::Pause => {
                        let _ = socket.send(Message::Text("p".into())).await;
                    }
                    GameServerCommand::Unpause => {
                        let _ = socket.send(Message::Text("P".into())).await;
                    }
                    GameServerCommand::Reset => {
                        let _ = socket.send(Message::Text("r".into())).await;
                    }
                    GameServerCommand::SetState(_) => todo!(),
                },
                // success receive state from game server
                Either::Right((Some(Ok(Message::Binary(bytes))), _)) => {
                    let mut g = GameState::new();
                    match g.update(&bytes) {
                        Ok(()) => state_sender.unbounded_send(g).unwrap(),
                        Err(e) => eprintln!("Error updating game state: {e:?}"),
                    }
                }
                Either::Left((None, _)) => panic!("Commands channel was closed"),
                Either::Right((None, _)) => {
                    eprintln!("Game server connection closed");
                    status(&app, |s| s.game_server_connected = false);
                    eprintln!("Retrying in 1 second...");
                    sleep(Duration::from_secs(1)).await;

                    s = None;
                }
                Either::Right((Some(Err(e)), _)) => {
                    eprintln!("Error receiving from game server: {e:?}");
                    status(&app, |s| s.game_server_connected = false);
                    s = None;
                }
                Either::Right((Some(Ok(message)), _)) => {
                    eprintln!("Game server sent strange message: {message:?}");
                    status(&app, |s| s.game_server_connected = false);
                    s = None;
                }
            }
        } else {
            if let Some(([a, b, c, d], p)) = addr {
                // try to connect to the address
                let addr = format!("ws://{a}.{b}.{c}.{d}:{p}");

                match connect_async(&addr).await {
                    Ok((ws_stream, _)) => {
                        s = Some(ws_stream);
                        status(&app, |s| s.game_server_connected = true);
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to game server: {e:?}");
                        status(&app, |s| s.game_server_connected = false);
                        eprintln!("Retrying in 1 second...");
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            } else {
                // wait for an address
                loop {
                    if let Some(GameServerCommand::Connect(x)) = commands.next().await {
                        addr = x;
                        break;
                    }
                }
            }
        }
    }
}
