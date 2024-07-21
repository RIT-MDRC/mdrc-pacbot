use crate::driving::{info, RobotInterTaskMessage, RobotTask, Task};
use crate::messages::{RobotToServerMessage, ServerToRobotMessage};
use crate::names::RobotName;
use core::fmt::Debug;
use embedded_io_async::{Read, Write};
use heapless::Vec;
use static_cell::StaticCell;

#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    pub ssid: [u8; 32],
    pub is_5g: bool,
}

pub trait RobotNetworkBehavior: RobotTask {
    type Error: Debug;
    type Socket<'a>: Read + Write
    where
        Self: 'a;

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
    ) -> Result<(), Self::Error>;

    /// Disconnect from any active wifi network
    async fn disconnect_wifi(&mut self) -> ();

    /// Accept a socket that meets the requirements. Close the previous one if one exists
    async fn tcp_accept<'a>(
        &mut self,
        port: u16,
        tx_buffer: &'a mut [u8; 5000],
        rx_buffer: &'a mut [u8; 5000],
    ) -> Result<Self::Socket<'a>, Self::Error>
    where
        Self: 'a;

    /// Dispose of the current socket
    async fn tcp_close<'a>(&mut self, socket: Self::Socket<'a>);

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.write_firmware
    async fn write_firmware(&mut self, offset: usize, data: &[u8]) -> Result<(), Self::Error>;

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.hash
    async fn hash_firmware(&mut self, update_len: u32, output: &mut [u8; 32]);

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.mark_updated
    async fn mark_firmware_updated(&mut self);

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.get_state
    async fn firmware_swapped(&mut self) -> bool;

    /// Reboot the microcontroller, as fully as possible
    async fn reboot(self);

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.mark_booted
    async fn mark_firmware_booted(&mut self);
}

pub async fn network_task<T: RobotNetworkBehavior>(mut network: T) -> Result<(), T::Error> {
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");
    info!("{} initialized", name);

    static TX_BUFFER: StaticCell<[u8; 5000]> = StaticCell::new();
    static RX_BUFFER: StaticCell<[u8; 5000]> = StaticCell::new();

    let tx_buffer = TX_BUFFER.init([0; 5000]);
    let rx_buffer = RX_BUFFER.init([0; 5000]);

    loop {
        if network.wifi_is_connected().await.is_none() {
            network
                .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
                .await?;
            info!("{} network connected", name);
        }

        match network.tcp_accept(1234, rx_buffer, tx_buffer).await {
            Ok(mut socket) => {
                info!("{} client connected", name);

                if let Err(_) = write(name, &mut socket, RobotToServerMessage::Name(name)).await {
                    info!("{} failed to send name", name);
                    continue;
                }

                info!("{} sent name", name);

                loop {
                    match read(name, &mut socket).await {
                        Err(_) => break,
                        Ok(ServerToRobotMessage::TargetVelocity(lin, ang)) => network
                            .send_message(
                                RobotInterTaskMessage::TargetVelocity(lin, ang),
                                Task::Motors,
                            )
                            .await
                            .unwrap(),
                        Ok(ServerToRobotMessage::FirmwareWritePart { offset, .. }) => {
                            let mut buf4 = [0; 4];
                            let mut buf = [0; 4096];
                            // the first number should be 4096
                            if let Ok(_) = socket.read_exact(&mut buf4).await {
                                let len = u32::from_be_bytes(buf4) as usize;
                                if len == 4096 {
                                    if let Ok(_) = socket.read_exact(&mut buf).await {
                                        if let Ok(_) = network.write_firmware(offset, &buf).await {
                                            let _ = write(
                                                name,
                                                &mut socket,
                                                RobotToServerMessage::ConfirmFirmwarePart {
                                                    offset,
                                                    len,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Ok(ServerToRobotMessage::CalculateFirmwareHash(len)) => {
                            let mut buf = Default::default();
                            network.hash_firmware(len, &mut buf).await;
                            let _ =
                                write(name, &mut socket, RobotToServerMessage::FirmwareHash(buf));
                        }
                        Ok(ServerToRobotMessage::MarkFirmwareUpdated) => {
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::MarkedFirmwareUpdated,
                            );
                        }
                        Ok(ServerToRobotMessage::IsFirmwareSwapped) => {
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::FirmwareIsSwapped(
                                    network.firmware_swapped().await,
                                ),
                            );
                        }
                        Ok(ServerToRobotMessage::MarkFirmwareBooted) => {
                            network.mark_firmware_booted().await;
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::MarkedFirmwareBooted,
                            );
                        }
                        Ok(ServerToRobotMessage::ReadyToStartUpdate) => {
                            let _ =
                                write(name, &mut socket, RobotToServerMessage::ReadyToStartUpdate);
                        }
                        Ok(ServerToRobotMessage::Reboot) => {
                            if let Ok(_) =
                                write(name, &mut socket, RobotToServerMessage::Rebooting).await
                            {
                                network.reboot().await;
                                unreachable!("o7")
                            }
                        }
                        Ok(ServerToRobotMessage::CancelFirmwareUpdate) => {}
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

    info!("{} listening for message...", name);
    // first read the length of the message (u32)
    let mut len_buf = [0; 4];
    match network.read(&mut len_buf).await {
        Ok(4) => (),
        _ => return Err(()),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    info!("{} got length {}", name, len);
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
