pub mod data;
pub mod motors;
pub mod network;
pub mod peripherals;

use crate::driving::motors::RobotMotorsBehavior;
use crate::driving::network::RobotNetworkBehavior;
use crate::driving::peripherals::RobotPeripheralsBehavior;
use crate::util::CrossPlatformInstant;

pub trait RobotBehavior: 'static {
    type Instant: CrossPlatformInstant + Default;

    type Motors: RobotMotorsBehavior;
    type Network: RobotNetworkBehavior;
    type Peripherals: RobotPeripheralsBehavior;
}
