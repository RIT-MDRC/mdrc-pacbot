use core::fmt::Debug;
use core::future::Future;

pub trait RobotBehavior {
    type SpawnError: Debug;

    fn spawn_wifi_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_motors_task(&mut self) -> Result<(), Self::SpawnError>;
    fn spawn_i2c_task(&mut self) -> Result<(), Self::SpawnError>;

    fn get_distance_sensor() -> impl Future<Output = ()> + Send {
        core::future::ready(())
    }
}

pub async fn start_robot<T: RobotBehavior>(mut robot: T) {
    robot.spawn_wifi_task().unwrap();
}

pub async fn wifi_task() {}
pub async fn motors_task() {}
pub async fn i2c_task() {}
