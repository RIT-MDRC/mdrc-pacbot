use std::io;

use async_std::net::{TcpListener, TcpStream};
use embedded_io_async::{ErrorType, Read, Write};
use futures::{AsyncReadExt, AsyncWriteExt};

use crate::driving::TaskChannels;
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};
use core_pb::names::RobotName;

pub struct SimNetwork {
    name: RobotName,
    channels: TaskChannels,
    network_connected: bool,
}

impl SimNetwork {
    pub fn new(name: RobotName, channels: TaskChannels) -> Self {
        Self {
            name,
            channels,
            network_connected: false,
        }
    }
}

#[derive(Debug)]
pub enum SimNetworkError {
    TcpAcceptFailed,
}

pub struct TcpStreamReadWrite {
    stream: TcpStream,
}

impl ErrorType for TcpStreamReadWrite {
    type Error = io::Error;
}

impl Read for TcpStreamReadWrite {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.stream.read(buf).await
    }
}

impl Write for TcpStreamReadWrite {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.stream.write(buf).await
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
    type Socket = TcpStreamReadWrite;

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
    ) -> Result<(), Self::Error> {
        self.network_connected = true;
        Ok(())
    }

    async fn disconnect_wifi(&mut self) {
        self.network_connected = false;
    }

    async fn tcp_accept(&mut self, port: u16) -> Result<Self::Socket, Self::Error> {
        match TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(listener) => match listener.accept().await {
                Err(e) => {
                    eprintln!("Error accepting socket: {e:?}")
                }
                Ok((stream, addr)) => {
                    println!("Client connected to a robot from {addr}");
                    return Ok(TcpStreamReadWrite { stream });
                }
            },
            Err(e) => {
                eprintln!("Error binding listener: {e:?}");
            }
        }
        Err(SimNetworkError::TcpAcceptFailed)
    }

    async fn tcp_close(&mut self, mut socket: Self::Socket) {
        let _ = socket.stream.close().await;
    }
}
