use crate::{status, App};
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::GuiToGameServerMessage;
use core_pb::{bin_decode, bin_encode};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::future::{select, Either};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub async fn listen_for_gui_clients(
    app: Arc<Mutex<App>>,
    incoming: UnboundedSender<GuiToGameServerMessage>,
    mut outgoing: UnboundedReceiver<ServerStatus>,
) -> ! {
    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let addr = format!("0.0.0.0:{GUI_LISTENER_PORT}");

    loop {
        *state.lock().unwrap() = HashMap::new();

        match TcpListener::bind(addr.clone()).await {
            Ok(listener) => {
                println!("Listening on: {addr}");

                loop {
                    let acc = listener.accept();
                    pin_mut!(acc);

                    match select(acc, outgoing.next()).await {
                        Either::Left((Ok((stream, addr)), _)) => {
                            tokio::spawn(handle_gui_client(
                                app.clone(),
                                state.clone(),
                                stream,
                                addr,
                                incoming.clone(),
                            ));
                        }
                        Either::Left((Err(e), _)) => {
                            eprintln!("Failed to accept gui client: {e:?}");
                            println!("Waiting 5 seconds for gui listener restart...");
                            sleep(Duration::from_secs(5)).await;
                            break;
                        }
                        Either::Right((Some(msg), _)) => {
                            let bytes = Message::Binary(bin_encode(msg).unwrap());
                            for (addr, c) in state.lock().unwrap().iter_mut() {
                                match c.unbounded_send(bytes.clone()) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to send message to gui client {addr}: {e:?}"
                                        );
                                    }
                                }
                            }
                        }
                        Either::Right((None, _)) => {
                            panic!("Outgoing network messages channel closed");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to bind: {e:?}");
                println!("Waiting 5 seconds for gui listener restart...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn handle_gui_client(
    app: Arc<Mutex<App>>,
    peer_map: PeerMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
    incoming_msg: UnboundedSender<GuiToGameServerMessage>,
) {
    println!("Incoming TCP connection from gui client: {}", addr);

    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Error during the gui client websocket handshake occurred: {e:?}");
            return;
        }
    };
    println!("WebSocket connection from gui client established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);
    status(&app, |s| {
        s.gui_clients += 1;
        println!("{}", s.gui_clients);
    });

    let (outgoing, incoming) = ws_stream.split();

    // Future to loop through incoming messages
    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!("Received a message from gui client {}", addr);
        match msg {
            Message::Binary(bytes) => match bin_decode(&bytes) {
                Ok((msg, _)) => incoming_msg.unbounded_send(msg).unwrap(),
                Err(e) => eprintln!("Error decoding message from {addr}: {e:?}"),
            },
            Message::Close(_) => {
                println!("gui client {addr} is closing the connection");
            }
            m => eprintln!("Received strange message from gui client {addr}: {m:?}"),
        }

        future::ok(())
    });

    // Future to send queued messages to socket
    let receive_from_others = rx.map(Ok).forward(outgoing);

    // Do both
    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("gui client {} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
    status(&app, |s| {
        println!("{}", s.gui_clients);
        s.gui_clients -= 1
    });
}
