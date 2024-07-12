use std::collections::HashMap;
use std::pin::pin;
use std::sync::{Arc, Mutex};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{select, FutureExt, StreamExt};
use simple_websockets::{Event, Message, Responder};

use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::GuiToGameServerMessage;
use core_pb::{bin_decode, bin_encode};

use crate::{status, App};

pub async fn listen_for_gui_clients(
    app: Arc<Mutex<App>>,
    incoming: UnboundedSender<GuiToGameServerMessage>,
    mut outgoing: UnboundedReceiver<ServerStatus>,
) -> ! {
    let event_hub = simple_websockets::launch(GUI_LISTENER_PORT).unwrap();
    println!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");

    let mut gui_clients = HashMap::new();

    loop {
        let event_fut = pin!(event_hub.poll_async());
        let outgoing_fut = pin!(outgoing.next());

        select! {
            event = event_fut.fuse() => {
                handle_event(&app, &incoming, &mut gui_clients, event);
            }
            outgoing_msg = outgoing_fut.fuse() => {
                let msg = Message::Binary(bin_encode(outgoing_msg.unwrap()).unwrap());
                for (id, responder) in &mut gui_clients {
                    if !responder.send(msg.clone()) {
                        eprintln!("Error sending status to client #{id}, already closed")
                    }
                }
            }
        }
    }
}

fn handle_event(
    app: &Arc<Mutex<App>>,
    incoming: &UnboundedSender<GuiToGameServerMessage>,
    gui_clients: &mut HashMap<u64, Responder>,
    event: Event,
) {
    match event {
        Event::Connect(id, responder) => {
            println!("Gui client #{id} connected");
            status(app, |s| {
                s.gui_clients += 1;
                println!("{} gui client(s) are connected", s.gui_clients);
            });
            gui_clients.insert(id, responder);
        }
        Event::Disconnect(id) => {
            println!("Gui client #{id} disconnected");
            status(app, |s| {
                s.gui_clients -= 1;
                println!("{} gui client(s) remaining", s.gui_clients);
            });
            gui_clients.remove(&id);
        }
        Event::Message(id, msg) => {
            println!("Received a message from gui client {}", id);
            match msg {
                Message::Binary(bytes) => match bin_decode(&bytes) {
                    Ok(msg) => incoming.unbounded_send(msg).unwrap(),
                    Err(e) => eprintln!(
                        "Error decoding message from {id}: {e:?}, {} bytes",
                        bytes.len()
                    ),
                },
                Message::Text(text) => {
                    eprintln!("Received strange message from gui client {id}: {text}")
                }
            }
        }
    }
}
