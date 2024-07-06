use crate::driving::{info, RobotTask};
use core::fmt::Debug;
use heapless::Vec;

#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    pub ssid: [u8; 32],
    pub is_5g: bool,
}

pub trait RobotNetworkBehavior: RobotTask {
    type Error: Debug;

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
}

pub async fn network_task<T: RobotNetworkBehavior>(mut network: T) -> Result<(), T::Error> {
    if network.wifi_is_connected().await.is_none() {
        network
            .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
            .await?;
        info!("Network connected!");
    }

    Ok(())
}
