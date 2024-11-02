//! Code that enables shared network behavior between the simulator and physical robots

use crate::driving::{error, info, RobotInterTaskMessage, RobotTaskMessenger, Task};
use crate::messages::{NetworkStatus, RobotToServerMessage, ServerToRobotMessage};
use crate::names::RobotName;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::pin::pin;
use embedded_io_async::{Read, Write};
use futures::future::{select, Either};
use heapless::Vec;

/// The network that the robot will try to join on startup
///
/// Password can be set via the optional environment variable WIFI_PASSWORD
pub const DEFAULT_NETWORK: &str = "MdrcPacbot";

/// Information gathered about nearby networks that might be used for display UI
#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    /// Network SSID
    pub ssid: [u8; 32],
    /// Whether the network is 5G
    pub is_5g: bool,
}

/// Functionality that robots with networking must support
pub trait RobotNetworkBehavior {
    /// Errors that network functions might generate
    type Error: Debug;
    /// The type that 'tcp accept' calls should return to communicate with the server
    type Socket<'a>: Read + Write
    where
        Self: 'a;
    /// Instant functionality that may be implemented differently on different platforms
    type Instant: CrossPlatformInstant;

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

    /// No required functionality - indicates that an update is about to begin
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

    /// Read (blocking) some bytes emitted by defmt
    fn read_logging_bytes(buf: &mut [u8]) -> Option<usize>;
}

struct NetworkData<T: RobotNetworkBehavior, M: RobotTaskMessenger> {
    name: RobotName,
    network: T,
    msgs: M,

    socket_failed: bool,
}

impl<T: RobotNetworkBehavior, M: RobotTaskMessenger> NetworkData<T, M> {
    async fn connect_wifi(&mut self) {
        while self.network.wifi_is_connected().await.is_none() {
            self.status(NetworkStatus::Connecting, None).await;
            loop {
                if let Ok(()) = self
                    .network
                    .connect_wifi(DEFAULT_NETWORK, option_env!("WIFI_PASSWORD"))
                    .await
                {
                    let ip = self.network.wifi_is_connected().await.unwrap_or([0; 4]);
                    self.status(NetworkStatus::Connected, Some(ip)).await;
                    break;
                }
                self.status(NetworkStatus::ConnectionFailed, None).await;
            }
            info!("{} network connected", self.name);
        }
    }

    async fn status(&mut self, status: NetworkStatus, ip: Option<[u8; 4]>) {
        self.msgs
            .send_blocking(
                RobotInterTaskMessage::NetworkStatus(status, ip),
                Task::Peripherals,
            )
            .await;
    }

    async fn send(&mut self, socket: &mut T::Socket<'_>, message: RobotToServerMessage) {
        let mut buf = [0; 1000];
        let len =
            match bincode::serde::encode_into_slice(message, &mut buf, bincode::config::standard())
            {
                Ok(len) => len,
                Err(_) => {
                    error!("{} failed to encode message", self.name);
                    self.socket_failed = true;
                    return;
                }
            };
        if write_bytes(socket, &buf[..len], false).await.is_err() {
            error!("{} failed to send message", self.name);
            self.socket_failed = true;
        }
    }

    async fn send_bytes(&mut self, socket: &mut T::Socket<'_>, bytes: &[u8]) {
        if write_bytes(socket, bytes, true).await.is_err() {
            self.socket_failed = true;
            // don't print here because that might cause infinite printing as it fails to send
        }
    }
}

/// The "main" method for the network task
pub async fn network_task<T: RobotNetworkBehavior + 'static, M: RobotTaskMessenger>(
    mut network: T,
    msgs: M,
) -> Result<(), T::Error> {
    info!("mac address: {:?}", network.mac_address().await);
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");
    info!("{} initialized", name);

    let mut net = NetworkData {
        name,
        network,
        msgs,

        socket_failed: false,
    };

    let mut tx_buffer = [0; 5000];
    let mut rx_buffer = [0; 5000];

    let mut logs_buffer = [0; 512];

    let mut utilization_monitor: UtilizationMonitor<50, T::Instant> =
        UtilizationMonitor::new(0.0, 0.0);
    utilization_monitor.start();

    let mut utilizations = [0.0; 3];

    loop {
        net.connect_wifi().await;

        match net
            .network
            .tcp_accept(name.port(), &mut rx_buffer, &mut tx_buffer)
            .await
        {
            Ok(mut socket) => {
                let mut socket_ok_time = T::Instant::now();

                let s = &mut socket;
                info!("{} client connected", name);

                net.send(s, RobotToServerMessage::Name(name)).await;
                if net.socket_failed {
                    error!("{} failed to send name", name);
                    continue;
                }

                info!("{} sent name", name);

                loop {
                    if net.socket_failed && socket_ok_time.elapsed().as_millis() >= 1_000 {
                        error!("{} dropping socket due to extended downtime", name);
                        break;
                    }
                    if !net.socket_failed {
                        socket_ok_time = T::Instant::now();
                    }

                    utilization_monitor.stop();
                    let event = next_event::<T, M>(name, &mut net.msgs, s).await;
                    utilization_monitor.start();

                    // emit logs if we can find any
                    while let Some(count) = T::read_logging_bytes(&mut logs_buffer) {
                        net.send_bytes(s, &logs_buffer[..count]).await;
                    }

                    match event {
                        Either::Right(RobotInterTaskMessage::Utilization(util, task)) => {
                            utilizations[task as usize] = util;
                            utilizations[Task::Wifi as usize] = utilization_monitor.utilization();
                            net.send(s, RobotToServerMessage::Utilization(utilizations))
                                .await;
                        }
                        Either::Right(RobotInterTaskMessage::ToServer(msg)) => {
                            net.send(s, msg).await;
                        }
                        Either::Right(RobotInterTaskMessage::Sensors(sensors)) => {
                            net.send(s, RobotToServerMessage::Sensors(sensors)).await;
                        }
                        Either::Right(_) => {}
                        Either::Left(Err(())) => break,
                        Either::Left(Ok(ServerToRobotMessage::Ping)) => {
                            net.send(s, RobotToServerMessage::Pong).await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::FrequentRobotItems(msg))) => {
                            net.msgs.send_or_drop(
                                RobotInterTaskMessage::FrequentServerToRobot(msg.clone()),
                                Task::Motors,
                            );
                            net.msgs.send_or_drop(
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
                            if s.read_exact(&mut buf4).await.is_ok() {
                                let len = u32::from_be_bytes(buf4) as usize;
                                info!("{} is receiving {} bytes", name, len);
                                if s.read_exact(&mut buf[..len]).await.is_ok()
                                    && net
                                        .network
                                        .write_firmware(offset, &buf[..len])
                                        .await
                                        .is_ok()
                                {
                                    net.send(
                                        s,
                                        RobotToServerMessage::ConfirmFirmwarePart { offset, len },
                                    )
                                    .await;
                                }
                            }
                        }
                        Either::Left(Ok(ServerToRobotMessage::CalculateFirmwareHash(len))) => {
                            let mut buf = Default::default();
                            net.network.hash_firmware(len, &mut buf).await;
                            net.send(s, RobotToServerMessage::FirmwareHash(buf)).await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::MarkFirmwareUpdated)) => {
                            net.network.mark_firmware_updated().await;
                            net.send(s, RobotToServerMessage::MarkedFirmwareUpdated)
                                .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::IsFirmwareSwapped)) => {
                            let swapped = net.network.firmware_swapped().await;
                            net.send(s, RobotToServerMessage::FirmwareIsSwapped(swapped))
                                .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::MarkFirmwareBooted)) => {
                            net.network.mark_firmware_booted().await;
                            net.send(s, RobotToServerMessage::MarkedFirmwareBooted)
                                .await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::ReadyToStartUpdate)) => {
                            net.network.prepare_firmware_update().await;
                            info!("{} is ready for an update", name);
                            net.send(s, RobotToServerMessage::ReadyToStartUpdate).await;
                        }
                        Either::Left(Ok(ServerToRobotMessage::Reboot)) => {
                            net.send(s, RobotToServerMessage::Rebooting).await;
                            net.network.reboot().await;
                            unreachable!("o7")
                        }
                        Either::Left(Ok(ServerToRobotMessage::CancelFirmwareUpdate)) => {}
                        Either::Left(Ok(ServerToRobotMessage::ResetAngle)) => {
                            net.msgs
                                .send_blocking(RobotInterTaskMessage::ResetAngle, Task::Peripherals)
                                .await;
                        }
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

async fn write_bytes<T: Write>(
    socket: &mut T,
    buf: &[u8],
    raw_bytes: bool,
) -> Result<(), T::Error> {
    // first write the length of the message (u32)
    let len_buf = if raw_bytes {
        buf.len() as u32 + 1
    } else {
        buf.len() as u32
    }
    .to_be_bytes();
    socket.write_all(&len_buf).await?;

    if raw_bytes {
        // write the bytes identifier
        socket.write_all(&[255]).await?;
    }
    // then write the message
    socket.write_all(buf).await
}
