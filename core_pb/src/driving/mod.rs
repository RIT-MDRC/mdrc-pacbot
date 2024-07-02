use core::fmt::Debug;
use core::future::Future;
use defmt::info;
use heapless::Vec;

pub trait RobotBehavior {
    type SpawnError: Debug;

    fn spawn_wifi_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_motors_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_i2c_task(&mut self) -> Result<(), Self::SpawnError>;
}

/// Entry point, once initialization is complete
pub async fn start_all_tasks<T: RobotBehavior>(mut robot: T) -> Result<(), T::SpawnError> {
    robot.spawn_wifi_task()?;
    robot.spawn_motors_task()?;
    robot.spawn_i2c_task()
}

#[derive(Copy, Clone)]
pub enum Task {
    Wifi,
    Motors,
    I2c,
}

/// Messages passed between the various tasks
#[derive(Copy, Clone)]
pub enum RobotInterTaskMessage {}

pub trait RobotTask {
    /// Send a message to all other tasks
    ///
    /// If the receiver's buffer is full, returns Err(())
    fn send_message(
        &mut self,
        message: RobotInterTaskMessage,
        to: Task,
    ) -> impl Future<Output = Result<(), ()>>;

    /// Receive a message from other tasks; may be cancelled
    fn receive_message(&mut self) -> impl Future<Output = RobotInterTaskMessage>;
}

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

pub trait RobotMotorsBehavior: RobotTask {}

pub trait RobotI2cBehavior: RobotTask {}

pub async fn wifi_task<T: RobotWifiBehavior>(mut network: T) -> Result<(), T::Error> {
    if network.wifi_is_connected().await.is_none() {
        network
            .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
            .await?;
        info!("Network connected!");
    }

    Ok(())
}

pub async fn motors_task<T: RobotMotorsBehavior>(_motors: T) {}

pub async fn i2c_task<T: RobotI2cBehavior>(_i2c: T) {}
