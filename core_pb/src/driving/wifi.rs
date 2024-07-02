use crate::driving::RobotTask;
use core::fmt::Debug;
use core::future::Future;
use defmt::info;
use heapless::Vec;

#[derive(Copy, Clone)]
pub struct NetworkScanInfo {
    pub ssid: [u8; 32],
    pub is_5g: bool,
}

pub trait RobotWifiBehavior: RobotTask {
    type Error: Debug;

    /// If the device is currently connected to a wifi network, its IP, else None
    fn wifi_is_connected(&self) -> impl Future<Output = Option<[u8; 4]>>;

    /// List information for up to `C` networks
    fn list_networks<const C: usize>(&mut self) -> impl Future<Output = Vec<NetworkScanInfo, C>>;

    /// Connect to a network with the given username/password. This method shouldn't return until
    /// the connection either completes or fails, but it shouldn't do any retries.
    ///
    /// This will only be called if [`RobotWifiBehavior::wifi_is_connected`] is `false`
    fn connect_wifi(
        &mut self,
        network: &str,
        password: Option<&str>,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    /// Disconnect from any active wifi network
    fn disconnect_wifi(&mut self) -> impl Future<Output = ()>;
}

pub async fn wifi_task<T: RobotWifiBehavior>(mut network: T) -> Result<(), T::Error> {
    if network.wifi_is_connected().await.is_none() {
        network
            .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
            .await?;
        info!("Network connected!");
    }

    Ok(())
}
