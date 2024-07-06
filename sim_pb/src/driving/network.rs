use crate::driving::TaskChannels;
use core_pb::driving::network::{NetworkScanInfo, RobotNetworkBehavior};
use core_pb::driving::{RobotInterTaskMessage, RobotTask, Task};

pub struct SimNetwork {
    channels: TaskChannels,
    network_connected: bool,
}

impl SimNetwork {
    pub fn new(channels: TaskChannels) -> Self {
        Self {
            channels,
            network_connected: false,
        }
    }
}

#[derive(Debug)]
pub enum SimNetworkError {}

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
}
