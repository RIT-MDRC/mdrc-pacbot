use async_std::net::{TcpListener, TcpStream};
use bevy::tasks::block_on;
use embedded_io_async::{ErrorType, Read, Write};
use futures::{AsyncReadExt, AsyncWriteExt};
use std::io;
use std::io::ErrorKind;

use crate::driving::TaskChannels;
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::names::RobotName;

pub struct SimNetwork {
    name: RobotName,
    channels: TaskChannels,
    socket: Option<TcpStream>,
    network_connected: bool,
}

impl SimNetwork {
    pub fn new(name: RobotName, channels: TaskChannels) -> Self {
        Self {
            name,
            channels,
            socket: None,
            network_connected: false,
        }
    }
}

#[derive(Debug)]
pub enum SimNetworkError {
    TcpAcceptFailed,
}

impl ErrorType for SimNetwork {
    type Error = io::Error;
}

impl Read for SimNetwork {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match &mut self.socket {
            Some(socket) => socket.read(buf).await,
            None => Err(io::Error::new(ErrorKind::ConnectionReset, "")),
        }
    }
}

impl Write for SimNetwork {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match &mut self.socket {
            Some(socket) => socket.write(buf).await,
            None => Err(io::Error::new(ErrorKind::ConnectionReset, "")),
        }
    }
}

impl RobotTask for SimNetwork {
    async fn send_message(&mut self, message: RobotInterTaskMessage, to: Task) -> Result<(), ()> {
        self.channels.send_message(message, to).await
    }

    async fn receive_message(&mut self) -> RobotInterTaskMessage {
        self.channels.receive_message().await
    }
}

impl RobotNetworkBehavior for SimNetwork {
    type Error = SimNetworkError;

    async fn mac_address(&mut self) -> [u8; 6] {
        self.name.mac_address()
    }

    async fn wifi_is_connected(&self) -> Option<[u8; 4]> {
        if self.network_connected {
            Some([55, 55, 55, 55])
        } else {
            None
        }
    }

    async fn list_networks<const C: usize>(&mut self) -> heapless::Vec<NetworkScanInfo, C> {
        heapless::Vec::new()
    }

    async fn connect_wifi(
        &mut self,
        _network: &str,
        _password: Option<&str>,
    ) -> Result<(), <Self as RobotNetworkBehavior>::Error> {
        self.network_connected = true;
        Ok(())
    }

    async fn disconnect_wifi(&mut self) {
        self.network_connected = false;
    }

    async fn tcp_accept(&mut self, port: u16) -> Result<(), <Self as RobotNetworkBehavior>::Error> {
        match TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(listener) => match listener.accept().await {
                Err(e) => {
                    eprintln!("Error accepting socket: {e:?}")
                }
                Ok((stream, addr)) => {
                    println!("Client connected to a robot from {addr}");
                    self.socket = Some(stream);
                    return Ok(());
                }
            },
            Err(e) => {
                eprintln!("Error binding listener: {e:?}");
            }
        }
        Err(SimNetworkError::TcpAcceptFailed)
    }

    async fn tcp_close(&mut self) {
        let _ = self.socket.take().map(|mut x| block_on(x.close()));
    }
}
