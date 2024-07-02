use crate::driving::RobotTask;
use core::fmt::Debug;

pub trait RobotI2cBehavior: RobotTask {
    type Error: Debug;
}

pub async fn i2c_task<T: RobotI2cBehavior>(_i2c: T) -> Result<(), T::Error> {
    Ok(())
}
