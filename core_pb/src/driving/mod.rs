pub mod data;
pub mod motors;
pub mod network;
pub mod peripherals;

use crate::driving::motors::RobotMotorsBehavior;
use crate::driving::network::RobotNetworkBehavior;
use crate::driving::peripherals::RobotPeripheralsBehavior;
use crate::util::CrossPlatformInstant;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::watch::{Receiver, Watch};

pub trait RobotBehavior: 'static {
    type Instant: CrossPlatformInstant + Default;

    type Motors: RobotMotorsBehavior;
    type Network: RobotNetworkBehavior;
    type Peripherals: RobotPeripheralsBehavior;
}

pub struct Watched<'a, M: RawMutex + 'static, T: Clone + 'static, const N: usize> {
    receiver: Receiver<'a, M, T, N>,
    data: T,
}

impl<'a, M: RawMutex, T: Clone, const N: usize> Watched<'a, M, T, N> {
    pub async fn new_receiver(watch: &'a Watch<M, T, N>) -> Self {
        let mut receiver = watch.receiver().unwrap();
        let data = receiver.get().await;
        Self { receiver, data }
    }

    pub fn get(&mut self) -> &T {
        if let Some(t) = self.receiver.try_changed() {
            self.data = t;
        }
        &self.data
    }
}
