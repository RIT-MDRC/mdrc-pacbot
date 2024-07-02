use core::fmt::Debug;
use core::future::Future;
use defmt::info;

pub trait RobotBehavior {
    type SpawnError: Debug;

    fn spawn_wifi_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_motors_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_i2c_task(&mut self) -> Result<(), Self::SpawnError>;
}

pub async fn start_all_tasks<T: RobotBehavior>(mut robot: T) -> Result<(), T::SpawnError> {
    robot.spawn_wifi_task()?;
    robot.spawn_motors_task()?;
    robot.spawn_i2c_task()
}

pub trait RobotWifiBehavior {
    type Error: Debug;

    /// Whether the device is currently connected to a wifi network
    fn wifi_is_connected(&self) -> impl Future<Output = bool>;

    /// Connect to a network with the given username/password. This method shouldn't return until
    /// the connection either completes or fails, but it shouldn't do any retries.
    ///
    /// This will only be called if [`RobotWifiBehavior::wifi_is_connected`] is `false`
    fn connect_wifi(
        &mut self,
        network: &str,
        password: Option<&str>,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

pub async fn wifi_task<T: RobotWifiBehavior>(mut network: T) -> Result<(), T::Error> {
    if !network.wifi_is_connected().await {
        network
            .connect_wifi("Fios-DwYj6", option_env!("WIFI_PASSWORD"))
            .await?;
        info!("Network connected!");
    }

    Ok(())
}

pub async fn motors_task() {}
pub async fn i2c_task() {}
