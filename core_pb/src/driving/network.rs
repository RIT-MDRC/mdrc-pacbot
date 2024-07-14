use crate::driving::{info, RobotTask};
use crate::messages::{RobotToServerMessage, ServerToRobotMessage};
use crate::names::RobotName;
use core::fmt::Debug;
use embedded_io_async::{Read, Write};
use heapless::Vec;

#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    pub ssid: [u8; 32],
    pub is_5g: bool,
}

pub trait RobotNetworkBehavior: RobotTask {
    type Error: Debug;
    type Socket: Read + Write;

    /// Get the device's mac address
    async fn mac_address(&self) -> [u16; 6];

    /// If the device is currently connected to a wifi network, its IP, else None
    async fn wifi_is_connected(&self) -> Option<[u8; 4]>;

    /// List information for up to `C` networks
    async fn list_networks<const C: usize>(&mut self) -> Vec<NetworkScanInfo, C>;

    /// Connect to a network with the given username/password. This method shouldn't return until
    /// the connection either completes or fails, but it shouldn't do any retries.
    ///
    /// This will only be called if [`RobotNetworkBehavior::wifi_is_connected`] is `false`
    async fn connect_wifi(
        &mut self,
        network: &str,
        password: Option<&str>,
    ) -> Result<(), Self::Error>;

    /// Disconnect from any active wifi network
    async fn disconnect_wifi(&mut self) -> ();

    /// Accept a socket that meets the requirements
    async fn tcp_accept(&mut self, port: u16) -> Result<Self::Socket, Self::Error>;

    /// Dispose of the given socket
    async fn tcp_close(&mut self, socket: Self::Socket);
}

pub async fn network_task<T: RobotNetworkBehavior>(mut network: T) -> Result<(), T::Error> {
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");

    loop {
        if network.wifi_is_connected().await.is_none() {
            network
                .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
                .await?;
            info!("{name} network connected");
        }

        match network.tcp_accept(name.port()).await {
            Ok(mut socket) => {
                info!("{name} client connected");

                if let Err(_) = write(name, &mut socket, RobotToServerMessage::Name(name)).await {
                    info!("{name} failed to send name");
                    continue;
                }

                info!("{name} sent name");

                let _ = read(name, &mut socket).await;
            }
            Err(_) => {
                info!("{name} failed to accept socket");
            }
        }
    }
}

async fn read<T: Read + Write>(
    name: RobotName,
    socket: &mut T,
) -> Result<ServerToRobotMessage, ()> {
    let mut buf = [0; 1000];

    // first read the length of the message (u32)
    let mut len_buf = [0; 4];
    match socket.read(&mut len_buf).await {
        Ok(4) => (),
        _ => return Err(()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    // then read the message
    match socket.read_exact(&mut buf[..len]).await {
        Ok(()) => {
            info!("{name} received message of length {len}");
            Err(())
        }
        _ => Err(()),
    }
}

async fn write<T: Read + Write>(
    name: RobotName,
    socket: &mut T,
    message: RobotToServerMessage,
) -> Result<(), ()> {
    let mut buf = [0; 1000];
    let len =
        match bincode::serde::encode_into_slice(message, &mut buf, bincode::config::standard()) {
            Ok(len) => len,
            Err(e) => {
                info!("{name} failed to encode message: {e}");
                return Err(());
            }
        };

    // first write the length of the message (u32)
    let len_buf = (len as u32).to_be_bytes();
    if let Err(_) = socket.write_all(&len_buf).await {
        return Err(());
    }

    // then write the message
    if let Err(_) = socket.write_all(&buf[..len]).await {
        return Err(());
    }

    Ok(())
}
