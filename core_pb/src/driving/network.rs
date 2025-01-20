use crate::constants::DEFAULT_NETWORK;
use crate::driving::data::SharedRobotData;
use crate::driving::RobotBehavior;
use crate::messages::robot_tcp::{write_tcp, BytesOrT, StatefulTcpReader, TcpError, TcpMessage};
use crate::messages::{
    ExtraImuData, ExtraOptsTypes, FrequentServerToRobot, MotorControlStatus, NetworkStatus,
    RobotToServerMessage, SensorData, ServerToRobotMessage,
};
use crate::names::RobotName;
use crate::util::utilization::UtilizationMonitor;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use core::pin::pin;
use core::sync::atomic::Ordering;
use core::time::Duration;
use defmt_or_log::{error, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::{Receiver, Sender};
use embedded_io_async::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;

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
        tx_buffer: &'a mut [u8; 5192],
        rx_buffer: &'a mut [u8; 5192],
    ) -> Result<Self::Socket<'a>, Self::Error>
    where
        Self: 'a;

    /// Dispose of the current socket
    async fn tcp_close<'a>(&mut self, socket: &mut Self::Socket<'a>);

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
    async fn reboot(&mut self);

    /// See https://docs.embassy.dev/embassy-boot/git/default/struct.FirmwareUpdater.html#method.mark_booted
    async fn mark_firmware_booted(&mut self);
}

struct ExpectedFirmwarePart {
    offset: usize,
    len: usize,
}

struct NetworkData<'a, R: RobotBehavior + 'a> {
    name: RobotName,
    network: R::Network,
    seq: u32,

    data: &'a SharedRobotData<R>,
    config: FrequentServerToRobot,
    config_sender: Sender<'a, CriticalSectionRawMutex, FrequentServerToRobot, 2>,
    network_status_sender: Sender<'a, CriticalSectionRawMutex, (NetworkStatus, Option<[u8; 4]>), 2>,
    sensors_receiver: Receiver<'a, CriticalSectionRawMutex, SensorData, 2>,
    motors_receiver: Receiver<'a, CriticalSectionRawMutex, MotorControlStatus, 2>,

    expected_firmware_part: Option<ExpectedFirmwarePart>,

    utilization_monitor: UtilizationMonitor<50, R::Instant>,

    socket_failed: bool,
    serialization_buf: [u8; 1024],
}

impl<R: RobotBehavior> NetworkData<'_, R> {
    async fn connect_wifi(&mut self) {
        while self.network.wifi_is_connected().await.is_none() {
            self.network_status_sender
                .send((NetworkStatus::Connecting, None));
            loop {
                if let Ok(()) = self
                    .network
                    .connect_wifi(DEFAULT_NETWORK, option_env!("WIFI_PASSWORD"))
                    .await
                {
                    let ip = self.network.wifi_is_connected().await.unwrap_or([0; 4]);
                    self.network_status_sender
                        .send((NetworkStatus::Connected, Some(ip)));
                    break;
                }
                self.network_status_sender
                    .send((NetworkStatus::ConnectionFailed, None));
            }
            info!("{} network connected", self.name);
        }
    }

    async fn send(
        &mut self,
        socket: &mut <R::Network as RobotNetworkBehavior>::Socket<'_>,
        message: RobotToServerMessage,
    ) {
        self.write_tcp(socket, BytesOrT::T(message)).await;
    }

    async fn send_bytes(
        &mut self,
        socket: &mut <R::Network as RobotNetworkBehavior>::Socket<'_>,
        bytes: &[u8],
    ) {
        self.write_tcp(socket, BytesOrT::Bytes(bytes)).await;
    }

    async fn write_tcp(
        &mut self,
        socket: &mut <R::Network as RobotNetworkBehavior>::Socket<'_>,
        msg: BytesOrT<'_, RobotToServerMessage>,
    ) {
        match write_tcp::<RobotToServerMessage>(&mut self.seq, msg, &mut self.serialization_buf) {
            Ok(len) => {
                if socket
                    .write_all(&self.serialization_buf[..len])
                    .await
                    .is_err()
                {
                    error!("{} failed to send message", self.name);
                    self.socket_failed = true;
                }
            }
            Err(_) => {
                error!("{} failed to send message", self.name);
                self.socket_failed = true;
            }
        }
    }

    // async fn handle_inter_task_message(
    //     &mut self,
    //     s: &mut R::Network::Socket<'_>,
    //     msg: RobotInterTaskMessage,
    // ) {
    //     match msg {
    //         RobotInterTaskMessage::Utilization(util, task) => {
    //             self.utilizations[task as usize] = util;
    //             self.utilizations[Task::Wifi as usize] = self.utilization_monitor.utilization();
    //             self.send(s, RobotToServerMessage::Utilization(self.utilizations))
    //                 .await;
    //         }
    //         RobotInterTaskMessage::ToServer(msg) => {
    //             self.send(s, msg).await;
    //         }
    //         RobotInterTaskMessage::Sensors(sensors) => {
    //             self.send(s, RobotToServerMessage::Sensors(sensors)).await;
    //         }
    //         _ => {}
    //     }
    // }

    async fn handle_server_message(
        &mut self,
        s: &mut <R::Network as RobotNetworkBehavior>::Socket<'_>,
        msg: &TcpMessage<'_, ServerToRobotMessage>,
    ) {
        let msg = match &msg.msg {
            BytesOrT::T(t) => t.clone(),
            BytesOrT::Bytes(b) => {
                if let Some(ExpectedFirmwarePart { offset, len }) = self.expected_firmware_part {
                    if b.len() == len && self.network.write_firmware(offset, b).await.is_ok() {
                        self.send(s, RobotToServerMessage::ConfirmFirmwarePart { offset, len })
                            .await;
                        self.expected_firmware_part = None;
                    }
                }
                return;
            }
        };
        match msg {
            ServerToRobotMessage::Ping => {
                self.send(s, RobotToServerMessage::Pong).await;
                let util = [
                    self.data.utilization[0].load(Ordering::Relaxed),
                    self.data.utilization[1].load(Ordering::Relaxed),
                    self.data.utilization[2].load(Ordering::Relaxed),
                ];
                self.send(s, RobotToServerMessage::Utilization(util)).await;
            }
            ServerToRobotMessage::FrequentRobotItems(msg) => {
                self.config = msg.clone();
                self.config_sender.send(msg);
            }
            ServerToRobotMessage::FirmwareWritePart { offset, len } => {
                self.expected_firmware_part = Some(ExpectedFirmwarePart { offset, len });
            }
            ServerToRobotMessage::CalculateFirmwareHash(len) => {
                let mut buf = Default::default();
                self.network.hash_firmware(len, &mut buf).await;
                self.send(s, RobotToServerMessage::FirmwareHash(buf)).await;
            }
            ServerToRobotMessage::MarkFirmwareUpdated => {
                self.network.mark_firmware_updated().await;
                self.send(s, RobotToServerMessage::MarkedFirmwareUpdated)
                    .await;
            }
            ServerToRobotMessage::IsFirmwareSwapped => {
                let swapped = self.network.firmware_swapped().await;
                self.send(s, RobotToServerMessage::FirmwareIsSwapped(swapped))
                    .await;
            }
            ServerToRobotMessage::MarkFirmwareBooted => {
                self.network.mark_firmware_booted().await;
                self.send(s, RobotToServerMessage::MarkedFirmwareBooted)
                    .await;
            }
            ServerToRobotMessage::ReadyToStartUpdate => {
                self.network.prepare_firmware_update().await;
                info!("{} is ready for an update", self.name);
                self.send(s, RobotToServerMessage::ReadyToStartUpdate).await;
            }
            ServerToRobotMessage::Reboot => {
                self.send(s, RobotToServerMessage::Rebooting).await;
                self.network.tcp_close(s).await;
                self.network.reboot().await;
                unreachable!("o7")
            }
            ServerToRobotMessage::CancelFirmwareUpdate => {}
            #[allow(deprecated)]
            ServerToRobotMessage::ExtraOpts(opts) => {
                opts.store_into(&self.data.extra_opts);
                let indicators = ExtraOptsTypes::load_from(&self.data.extra_indicators);
                self.send(s, RobotToServerMessage::ReceivedExtraOpts(opts))
                    .await;
                self.send(s, RobotToServerMessage::ExtraIndicators(indicators))
                    .await;
            }
        }
    }

    async fn handle_until_broken(
        &mut self,
        s: &mut <R::Network as RobotNetworkBehavior>::Socket<'_>,
    ) {
        let mut logs_buffer = [0; 512];
        let mut stateful_tcp_reader = StatefulTcpReader::new();
        let mut socket_ok_time = R::Instant::default();

        info!("{} client connected", self.name);

        self.send(s, RobotToServerMessage::Name(self.name)).await;
        if self.socket_failed {
            error!("{} failed to send name", self.name);
            return;
        }

        info!("{} sent name", self.name);

        loop {
            if self.socket_failed && socket_ok_time.elapsed().as_millis() >= 1_000 {
                error!("{} dropping socket due to extended downtime", self.name);
                return;
            }
            if !self.socket_failed {
                socket_ok_time = R::Instant::default();
            }

            // emit logs if we can find any
            while let Ok(count) = self.data.defmt_logs.try_read(&mut logs_buffer) {
                if count == 0 {
                    break;
                }
                self.send_bytes(s, &logs_buffer[..count]).await;
            }

            self.utilization_monitor.stop();
            let event = next_event::<R::Network, R::Instant>(
                &mut self.sensors_receiver,
                &mut self.motors_receiver,
                &self.data.sig_extra_imu_data,
                s,
                &mut stateful_tcp_reader,
            )
            .await;
            self.utilization_monitor.start();

            match event {
                NetworkEvent::TimedOut => {}
                NetworkEvent::ServerToRobot(Ok(msg)) => self.handle_server_message(s, &msg).await,
                NetworkEvent::ServerToRobot(Err(_e)) => {
                    // error!("Socket failed with error: {:?}", e);
                    break;
                }
                NetworkEvent::SensorData(data) => {
                    self.send(s, RobotToServerMessage::Sensors(data)).await
                }
                NetworkEvent::MotorData(data) => {
                    self.send(
                        s,
                        RobotToServerMessage::MotorControlStatus((
                            self.data.created_at.elapsed(),
                            data,
                        )),
                    )
                    .await
                }
                NetworkEvent::ExtraImuData(data) => {
                    self.send(s, RobotToServerMessage::ExtraImuData(data)).await
                }
            }
        }
    }
}

/// The "main" method for the network task
pub async fn network_task<R: RobotBehavior>(data: &SharedRobotData<R>, mut network: R::Network) {
    info!("mac address: {:?}", network.mac_address().await);
    let name = RobotName::from_mac_address(&network.mac_address().await)
        .expect("Unrecognized mac address");
    info!("{} initialized", name);

    let mut net = NetworkData {
        name,
        network,
        data,
        config: FrequentServerToRobot::new(name),
        seq: 0,

        config_sender: data.config.sender(),
        network_status_sender: data.network_status.sender(),

        sensors_receiver: data.sensors.receiver().unwrap(),
        motors_receiver: data.motor_control.receiver().unwrap(),
        expected_firmware_part: None,

        utilization_monitor: UtilizationMonitor::new(0.0, 0.0),

        socket_failed: false,
        serialization_buf: [0; 1024],
    };

    net.utilization_monitor.start();

    let mut tx_buffer = [0; 5192];
    let mut rx_buffer = [0; 5192];

    loop {
        net.connect_wifi().await;

        match net
            .network
            .tcp_accept(name.port(), &mut rx_buffer, &mut tx_buffer)
            .await
        {
            Ok(mut socket) => net.handle_until_broken(&mut socket).await,
            Err(_) => {
                info!("{} failed to accept socket", name);
            }
        }
    }
}

enum NetworkEvent<'reader> {
    TimedOut,
    ServerToRobot(Result<TcpMessage<'reader, ServerToRobotMessage>, TcpError>),
    SensorData(SensorData),
    MotorData(MotorControlStatus),
    ExtraImuData(ExtraImuData),
}

async fn next_event<'reader, R: RobotNetworkBehavior, I: CrossPlatformInstant>(
    sensors: &mut Receiver<'_, CriticalSectionRawMutex, SensorData, 2>,
    motors: &mut Receiver<'_, CriticalSectionRawMutex, MotorControlStatus, 2>,
    imu: &Signal<CriticalSectionRawMutex, ExtraImuData>,
    socket: &mut R::Socket<'_>,
    stateful_tcp_reader: &'reader mut StatefulTcpReader,
) -> NetworkEvent<'reader> {
    let f1 = pin!(stateful_tcp_reader.read_socket(socket));
    let f2 = pin!(sensors.changed());
    let f3 = pin!(motors.changed());
    let f4 = pin!(imu.wait());
    select_biased! {
        msg = f1.fuse() => NetworkEvent::ServerToRobot(msg),
        data = f2.fuse() => NetworkEvent::SensorData(data),
        data = f3.fuse() => NetworkEvent::MotorData(data),
        data = f4.fuse() => NetworkEvent::ExtraImuData(data),
        _ = I::sleep(Duration::from_millis(1000 / 30)).fuse() => NetworkEvent::TimedOut,
    }
}
