pub mod data;
pub mod motors;
pub mod network;
pub mod peripherals;

use crate::driving::motors::RobotMotorsBehavior;
use crate::driving::network::RobotNetworkBehavior;
use crate::driving::peripherals::RobotPeripheralsBehavior;
use crate::util::CrossPlatformInstant;
use core::time::Duration;

pub trait RobotBehavior: 'static {
    type Instant: CrossPlatformInstant + Default;

    type Motors: RobotMotorsBehavior;
    type Network: RobotNetworkBehavior;
    type Peripherals: RobotPeripheralsBehavior;
}

#[derive(Default)]
pub struct Ticker<I: Default>(I);

impl<I: CrossPlatformInstant + Default> Ticker<I> {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn tick(&mut self, interval: Duration, min_wait: Duration) {
        if self.0.elapsed() > interval {
            I::sleep(min_wait).await;
        } else {
            let t = Duration::max(interval - self.0.elapsed(), min_wait);
            I::sleep(t).await;
        }
        self.0 = I::default()
    }
}
