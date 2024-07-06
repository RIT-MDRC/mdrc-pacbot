use crate::driving::RobotTask;
use core::fmt::Debug;

pub trait RobotPeripheralsBehavior: RobotTask {
    type Error: Debug;
}

pub async fn peripherals_task<T: RobotPeripheralsBehavior>(
    _peripherals: T,
) -> Result<(), T::Error> {
    Ok(())
}
