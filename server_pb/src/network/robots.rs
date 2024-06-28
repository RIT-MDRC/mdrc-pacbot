use crate::{status, App};
use core_pb::messages::server_status::RobotStatus;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub type IpPort = ([u8; 4], u16);

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum RobotConnectionMessage {
    Connect(IpPort),
    Reconnect(IpPort),
    Disconnect(IpPort),
}

// pub async fn manage_robots(ip_ports: UnboundedReceiver<[IpPort]>, app: Arc<Mutex<App>>) {}

fn robot_status<F>(app: &Arc<Mutex<App>>, ip_port: IpPort, changes: F)
where
    F: FnOnce(&mut RobotStatus),
{
    status(app, |s| {
        for (r, robot_settings) in s.settings.robots.iter().enumerate() {
            if (robot_settings.ipv4, robot_settings.tcp_port) == ip_port {
                changes(&mut s.robots[r]);
                return;
            }
        }
    })
}

async fn manage_robot(app: Arc<Mutex<App>>, ip_port: IpPort) -> ! {
    let mut socket = None;

    loop {
        if let Some(socket) = socket {
            // match socket.read
            todo!()
        } else {
            // try to reconnect to robot
            let ([a, b, c, d], e) = ip_port;
            let addr = format!("{a}.{b}.{c}.{d}:{e}");
            match TcpStream::connect(addr.clone()).await {
                Ok(s) => {
                    println!("Connected to robot at {addr}");
                    robot_status(&app, ip_port, |s| s.connected = true);
                    socket = Some(s);
                }
                Err(e) => eprintln!("Error connecting to robot at {addr}: {e:?}"),
            }
        }
    }
}
