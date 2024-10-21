use crate::driving::{info, RobotInterTaskMessage, RobotTaskMessenger, Task};
use crate::messages::{NetworkStatus, RobotToServerMessage, ServerToRobotMessage};
use crate::names::RobotName;
use core::fmt::Debug;
use core::pin::pin;
use embedded_io_async::{Read, Write};
use futures::future::{select, Either};
use heapless::Vec;

pub const DEFAULT_NETWORK: &str = "The Province";

#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    pub ssid: [u8; 32],
    pub is_5g: bool,
}

/// Functionality that robots with networking must support
pub trait RobotNetworkBehavior {
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
    async fn disconnect_wifi(&mut self);

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

    async fn prepare_firmware_update(&mut self);

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

/// The "main" method for the network task
pub async fn network_task<T: RobotNetworkBehavior, M: RobotTaskMessenger>(
    mut network: T,
    mut msgs: M,
) -> Result<(), T::Error> {
    info!("mac address: {:?}", network.mac_address().await);
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");
    info!("{} initialized", name);

    let mut tx_buffer = [0; 5000];
    let mut rx_buffer = [0; 5000];

    loop {
        if network.wifi_is_connected().await.is_none() {
            msgs.send_blocking(
                RobotInterTaskMessage::NetworkStatus(NetworkStatus::Connecting, None),
                Task::Peripherals,
            )
            .await;
            loop {
                if let Ok(()) = network
                    .connect_wifi(DEFAULT_NETWORK, option_env!("WIFI_PASSWORD"))
                    .await
                {
                    let ip = network.wifi_is_connected().await.unwrap_or([0; 4]);
                    msgs.send_blocking(
                        RobotInterTaskMessage::NetworkStatus(NetworkStatus::Connected, Some(ip)),
                        Task::Peripherals,
                    )
                    .await;
                    break;
                }
                msgs.send_blocking(
                    RobotInterTaskMessage::NetworkStatus(NetworkStatus::ConnectionFailed, None),
                    Task::Peripherals,
                )
                .await;
            }
            info!("{} network connected", name);
        }

        match network
            .tcp_accept(name.port(), &mut rx_buffer, &mut tx_buffer)
            .await
        {
            Ok(mut socket) => {
                info!("{} client connected", name);

                if write(name, &mut socket, RobotToServerMessage::Name(name))
                    .await
                    .is_err()
                {
                    info!("{} failed to send name", name);
                    continue;
                }

                info!("{} sent name", name);

                loop {
                    match next_event::<T, M>(name, &mut msgs, &mut socket).await {
                        Either::Right(RobotInterTaskMessage::ToServer(msg)) => {
                            if write(name, &mut socket, msg).await.is_err() {
                                break;
                            }
                        }
                        Either::Right(RobotInterTaskMessage::Sensors(sensors)) => {
                            if write(name, &mut socket, RobotToServerMessage::Sensors(sensors))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Either::Right(_) => {}
                        Either::Left(Err(())) => break,
                        Either::Left(Ok(ServerToRobotMessage::FrequentRobotItems(msg))) => {
                            msgs.send_or_drop(
                                RobotInterTaskMessage::FrequentServerToRobot(msg.clone()),
                                Task::Motors,
                            );
                            msgs.send_or_drop(
                                RobotInterTaskMessage::FrequentServerToRobot(msg),
                                Task::Peripherals,
                            );
                        }
                        Either::Left(Ok(ServerToRobotMessage::FirmwareWritePart {
                            offset,
                            ..
                        })) => {
                            let mut buf4 = [0; 4];
                            let mut buf = [0; 4096];
                            // the first number should be 4096
                            if socket.read_exact(&mut buf4).await.is_ok() {
                                let len = u32::from_be_bytes(buf4) as usize;
                                info!("{} is receiving {} bytes", name, len);
                                if socket.read_exact(&mut buf[..len]).await.is_ok()
                                    && network.write_firmware(offset, &buf[..len]).await.is_ok()
                                {
                                    let _ = write(
                                        name,
                                        &mut socket,
                                        RobotToServerMessage::ConfirmFirmwarePart { offset, len },
                                    )
                                    .await;
                                }
                            }
                        }
                        Either::Left(Ok(ServerToRobotMessage::CalculateFirmwareHash(len))) => {
                            let mut buf = Default::default();
                            network.hash_firmware(len, &mut buf).await;
                            let _ =
                                write(name, &mut socket, RobotToServerMessage::FirmwareHash(buf))
                                    .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::MarkFirmwareUpdated)) => {
                            network.mark_firmware_updated().await;
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::MarkedFirmwareUpdated,
                            )
                            .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::IsFirmwareSwapped)) => {
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::FirmwareIsSwapped(
                                    network.firmware_swapped().await,
                                ),
                            )
                            .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::MarkFirmwareBooted)) => {
                            network.mark_firmware_booted().await;
                            let _ = write(
                                name,
                                &mut socket,
                                RobotToServerMessage::MarkedFirmwareBooted,
                            )
                            .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::ReadyToStartUpdate)) => {
                            network.prepare_firmware_update().await;
                            info!("{} is ready for an update", name);
                            let _ =
                                write(name, &mut socket, RobotToServerMessage::ReadyToStartUpdate)
                                    .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::Reboot)) => {
                            if write(name, &mut socket, RobotToServerMessage::Rebooting)
                                .await
                                .is_ok()
                            {
                                network.reboot().await;
                                unreachable!("o7")
                            }
                        }
                        Either::Left(Ok(ServerToRobotMessage::CancelFirmwareUpdate)) => {}
                    }
                }
            }
            Err(_) => {
                info!("{} failed to accept socket", name);
            }
        }
    }
}

async fn next_event<'a, T: RobotNetworkBehavior, M: RobotTaskMessenger>(
    name: RobotName,
    msgs: &mut M,
    mut socket: &mut T::Socket<'a>,
) -> Either<Result<ServerToRobotMessage, ()>, RobotInterTaskMessage> {
    // if the socket has data, we need to be sure to completely read it, or we'll only have half
    // a message
    let mut len_buf = [0; 4];
    match select(
        pin!(socket.read(&mut len_buf)),
        pin!(msgs.receive_message()),
    )
    .await
    {
        Either::Left((read_result, _)) => match read_result {
            Ok(4) => (),
            _ => return Either::Left(Err(())),
        },
        Either::Right((msg, _)) => return Either::Right(msg),
    };
    // after dropping future
    let len = u32::from_be_bytes(len_buf) as usize;
    Either::Left(read_rest(name, &mut socket, len).await)
}

async fn read_rest<T: Read>(
    _name: RobotName,
    network: &mut T,
    len: usize,
) -> Result<ServerToRobotMessage, ()> {
    let mut buf = [0; 6000];

    // info!("{} listening for message...", name);
    // after read the length of the message (u32)
    // info!("{} got length {}", name, len);
    // then read the message
    if len > buf.len() {
        return Err(());
    }
    match network.read_exact(&mut buf[..len]).await {
        Ok(()) => {
            // info!("{} received message of length {}", name, len);
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
    if network.write_all(&len_buf).await.is_err() {
        info!("{} error writing to socket", name);
        return Err(());
    }

    // then write the message
    if network.write_all(&buf[..len]).await.is_err() {
        info!("{} error writing to socket", name);
        return Err(());
    }

    // info!("{} sent message of length {}", name, len);

    Ok(())
}
