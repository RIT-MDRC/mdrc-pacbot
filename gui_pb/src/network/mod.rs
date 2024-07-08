mod websocket;

use crate::colors::{TRANSLUCENT_GREEN_COLOR, TRANSLUCENT_RED_COLOR, TRANSLUCENT_YELLOW_COLOR};
use crate::network::websocket::CrossPlatformWebsocket;
use crate::App;
use core_pb::bin_encode;
use core_pb::messages::GuiToGameServerMessage;
use eframe::egui::Color32;
use tungstenite::Message;
use web_time::{Duration, Instant};

#[derive(Default)]
pub struct NetworkData {
    mdrc_server_socket: Option<CrossPlatformWebsocket>,
    mdrc_server_last_time: Option<Instant>,
    mdrc_server_status: NetworkStatus,
    last_ip_port_attempt: Option<(Instant, [u8; 4], u16)>,
}

impl NetworkData {
    pub fn status(&self) -> NetworkStatus {
        self.mdrc_server_status
    }
}

#[derive(Copy, Clone, Default)]
pub enum NetworkStatus {
    /// Settings dictate that a connection should not be made
    #[default]
    NotConnected,
    /// A websocket cannot be established
    ConnectionFailed,
    /// After a websocket is established, but before a status message is received
    Connecting,
    /// After a status message is received
    Connected,
}

impl From<NetworkStatus> for Color32 {
    fn from(value: NetworkStatus) -> Self {
        match value {
            NetworkStatus::NotConnected => TRANSLUCENT_RED_COLOR,
            NetworkStatus::ConnectionFailed => TRANSLUCENT_RED_COLOR,
            NetworkStatus::Connecting => TRANSLUCENT_YELLOW_COLOR,
            NetworkStatus::Connected => TRANSLUCENT_GREEN_COLOR,
        }
    }
}

impl App {
    pub fn manage_network(&mut self) {
        if !self.data.ui_settings.connect_mdrc_server {
            // reset socket and status information
            self.data.network_data = NetworkData::default();
            return;
        }

        if let Some(socket) = &mut self.data.network_data.mdrc_server_socket {
            if socket.can_read() {
                // todo send settings/commands/keys
                if self.data.server_status.settings != self.data.settings {
                    socket
                        .send(
                            bin_encode(GuiToGameServerMessage::Settings(
                                self.data.settings.clone(),
                            ))
                            .unwrap(),
                        )
                        .unwrap();
                    self.data.server_status.settings = self.data.settings.clone();
                }

                // read status messages from server
                while let Ok(Message::Binary(m)) = socket.read() {
                    match bincode::serde::decode_from_slice(&m, bincode::config::standard()) {
                        Ok((status, _)) => {
                            self.data.server_status = status;
                            self.data.settings = self.data.server_status.settings.clone();
                            self.data.network_data.mdrc_server_last_time = Some(Instant::now());
                            self.data.network_data.mdrc_server_status = NetworkStatus::Connected;
                        }
                        Err(e) => eprintln!("Failed to decode status from server: {e:?}"),
                    }
                }

                // as long as we've received a status recently, we can be done here
                if let Some(t) = self.data.network_data.mdrc_server_last_time {
                    // this socket has produced at least one status - ensure the last one was recent
                    if t.elapsed() < Duration::from_secs(1) {
                        return;
                    }
                    // if it wasn't, continue on to replace the socket
                } else {
                    // this socket hasn't produced a status yet
                    // as long as it has only been alive for a short time, that's fine
                    if let Some((t2, _, _)) = self.data.network_data.last_ip_port_attempt {
                        if t2.elapsed() < Duration::from_secs(1) {
                            return;
                        }
                        // if it wasn't, continue on to replace the socket
                    } else {
                        eprintln!("WARNING: Had a socket without an attempted connection!");
                    }
                }
            }
        }

        // reset socket and status information, in case of broken socket
        self.data.network_data = NetworkData::default();

        // socket is not currently connected
        // have we tried the current IP/port recently?
        if let Some((t, ip, port)) = self.data.network_data.last_ip_port_attempt {
            if port == self.data.ui_settings.mdrc_server_ws_port
                && ip == self.data.ui_settings.mdrc_server_ipv4
                && t.elapsed() < Duration::from_millis(500)
            {
                // we have tried this IP/port recently; we'll try again later
                self.data.network_data.mdrc_server_status = NetworkStatus::ConnectionFailed;
                return;
            } else {
                // either the ip/port settings changed, or enough time has elapsed to try again
                self.data.network_data.last_ip_port_attempt = None;
            }
        }

        // we should try to reconnect
        println!("Attempting to connect to server...");
        self.data.network_data.mdrc_server_status = NetworkStatus::Connecting;

        let ip = self.data.ui_settings.mdrc_server_ipv4;
        let port = self.data.ui_settings.mdrc_server_ws_port;
        self.data.network_data.last_ip_port_attempt = Some((Instant::now(), ip, port));
        let [a, b, c, d] = ip;

        match CrossPlatformWebsocket::connect(format!("{a}.{b}.{c}.{d}:{port}")) {
            Ok(socket) => {
                println!("Connected successfully");
                self.data.network_data.mdrc_server_socket = Some(socket);
            }
            Err(e) => {
                eprintln!("Failed to establish TCP connection: {e:?}");
                self.data.network_data.mdrc_server_status = NetworkStatus::ConnectionFailed;
            }
        }
    }
}
