use crate::RobotToSimulationMessage;
use async_channel::Sender;
use async_std::io::{ReadExt, WriteExt};
use async_std::net::{TcpListener, TcpStream};
use async_std::task::sleep;
use bevy::prelude::{error, info};
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use core_pb::names::RobotName;
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};
use std::io;
use std::io::ErrorKind;
use std::time::Duration;

pub struct SimNetwork {
    name: RobotName,
    sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
    network_connected: bool,

    firmware_swapped: bool,
}

impl SimNetwork {
    pub fn new(
        name: RobotName,
        firmware_swapped: bool,
        sim_tx: Sender<(RobotName, RobotToSimulationMessage)>,
    ) -> Self {
        Self {
            name,
            sim_tx,
            network_connected: false,
            firmware_swapped,
        }
    }
}

#[derive(Debug)]
pub enum SimNetworkError {
    TcpAcceptFailed,
}

pub struct TcpStreamReadWrite(TcpStream);

impl ErrorType for TcpStreamReadWrite {
    type Error = io::Error;
}

impl Read for TcpStreamReadWrite {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
        self.0
            .read_exact(buf)
            .await
            .map_err(|_| ReadExactError::Other(io::Error::new(ErrorKind::Other, "")))
    }
}

impl Write for TcpStreamReadWrite {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await
    }
}

impl RobotNetworkBehavior for SimNetwork {
    type Error = SimNetworkError;
    type Socket<'a> = TcpStreamReadWrite where Self: 'a;

    async fn mac_address(&mut self) -> [u8; 6] {
        self.name.mac_address()
    }

    async fn wifi_is_connected(&self) -> Option<[u8; 4]> {
        if self.network_connected {
            Some([127, 0, 0, 1])
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
        sleep(Duration::from_secs(1)).await;
        Ok(())
    }

    async fn disconnect_wifi(&mut self) {
        self.network_connected = false;
    }

    async fn tcp_accept<'a>(
        &mut self,
        port: u16,
        _tx: &'a mut [u8; 5000],
        _rx: &'a mut [u8; 5000],
    ) -> Result<Self::Socket<'a>, <Self as RobotNetworkBehavior>::Error>
    where
        Self: 'a,
    {
        info!("{} listening on {port}!", self.name);
        match TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(listener) => match listener.accept().await {
                Err(e) => {
                    error!("Error accepting socket: {e:?}")
                }
                Ok((stream, addr)) => {
                    info!("Client connected to a {} from {addr}", self.name);
                    return Ok(TcpStreamReadWrite(stream));
                }
            },
            Err(e) => {
                error!("Error binding listener: {e:?}");
            }
        }
        Err(SimNetworkError::TcpAcceptFailed)
    }

    async fn tcp_close<'a>(&mut self, _socket: Self::Socket<'a>) {}

    async fn prepare_firmware_update(&mut self) {}

    async fn write_firmware(&mut self, _offset: usize, _data: &[u8]) -> Result<(), Self::Error> {
        sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    async fn hash_firmware(&mut self, _update_len: u32, _output: &mut [u8; 32]) {
        sleep(Duration::from_millis(50)).await;
    }

    async fn mark_firmware_updated(&mut self) {
        self.sim_tx
            .send((self.name, RobotToSimulationMessage::MarkFirmwareUpdated))
            .await
            .unwrap();
    }

    async fn firmware_swapped(&mut self) -> bool {
        self.firmware_swapped
    }

    async fn reboot(self) {
        self.sim_tx
            .send((self.name, RobotToSimulationMessage::Reboot))
            .await
            .unwrap();
        sleep(Duration::from_secs(99999)).await
    }

    async fn mark_firmware_booted(&mut self) {
        self.firmware_swapped = false;
    }
}
