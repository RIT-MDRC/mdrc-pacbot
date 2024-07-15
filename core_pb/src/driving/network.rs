use crate::driving::{info, RobotInterTaskMessage, RobotTask, Task};
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

pub trait RobotNetworkBehavior: RobotTask + Read + Write {
    type Error: Debug;

    /// Get the device's mac address
    async fn mac_address(&mut self) -> [u8; 6];

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
    ) -> Result<(), <Self as RobotNetworkBehavior>::Error>;

    /// Disconnect from any active wifi network
    async fn disconnect_wifi(&mut self) -> ();

    /// Accept a socket that meets the requirements. Close the previous one if one exists
    async fn tcp_accept(&mut self, port: u16) -> Result<(), <Self as RobotNetworkBehavior>::Error>;

    /// Dispose of the current socket, if one exists
    async fn tcp_close(&mut self);
}

pub async fn network_task<T: RobotNetworkBehavior>(
    mut network: T,
) -> Result<(), <T as RobotNetworkBehavior>::Error> {
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");

    loop {
        if network.wifi_is_connected().await.is_none() {
            network
                .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
                .await?;
            info!("{} network connected", name);
        }

        match network.tcp_accept(name.port()).await {
            Ok(_) => {
                info!("{} client connected", name);

                if let Err(_) = write(name, &mut network, RobotToServerMessage::Name(name)).await {
                    info!("{} failed to send name", name);
                    continue;
                }

                info!("{} sent name", name);

                loop {
                    match read(name, &mut network).await {
                        Err(_) => break,
                        Ok(ServerToRobotMessage::TargetVelocity(lin, ang)) => network
                            .send_message(
                                RobotInterTaskMessage::TargetVelocity(lin, ang),
                                Task::Motors,
                            )
                            .await
                            .unwrap(),
                    }
                }
            }
            Err(_) => {
                info!("{} failed to accept socket", name);
            }
        }
    }
}

async fn read<T: Read>(name: RobotName, network: &mut T) -> Result<ServerToRobotMessage, ()> {
    let mut buf = [0; 1000];

    // first read the length of the message (u32)
    let mut len_buf = [0; 4];
    match network.read(&mut len_buf).await {
        Ok(4) => (),
        _ => return Err(()),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    // then read the message
    match network.read_exact(&mut buf[..len]).await {
        Ok(()) => {
            info!("{} received message of length {}", name, len);
            match bincode::serde::decode_from_slice(&buf[..len], bincode::config::standard()) {
                Ok((msg, _)) => Ok(msg),
                Err(_) => {
                    info!("Failed to decode message");
                    Err(())
                }
            }
        }
        _ => Err(()),
    }
}

async fn write<T: Write>(
    name: RobotName,
    network: &mut T,
    message: RobotToServerMessage,
) -> Result<(), ()> {
    let mut buf = [0; 1000];
    let len =
        match bincode::serde::encode_into_slice(message, &mut buf, bincode::config::standard()) {
            Ok(len) => len,
            Err(_) => {
                info!("{} failed to encode message", name);
                return Err(());
            }
        };

    // first write the length of the message (u32)
    let len_buf = (len as u32).to_be_bytes();
    if let Err(_) = network.write_all(&len_buf).await {
        return Err(());
    }

    // then write the message
    if let Err(_) = network.write_all(&buf[..len]).await {
        return Err(());
    }

    Ok(())
}
