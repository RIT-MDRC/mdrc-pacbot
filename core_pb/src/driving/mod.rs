pub mod data;
pub mod motors;
pub mod network;
pub mod peripherals;

#[cfg(feature = "defmt")]
pub(crate) use defmt::*;
#[cfg(feature = "log")]
pub(crate) use log::*;

use crate::driving::motors::RobotMotorsBehavior;
use crate::driving::network::RobotNetworkBehavior;
use crate::driving::peripherals::RobotPeripheralsBehavior;
use crate::util::CrossPlatformInstant;
use core::fmt::Debug;
use embassy_time::Instant;
use std::ops::{Deref, DerefMut};

pub trait RobotBehavior {
    type PeripheralsError: Debug;

    type Motors: RobotMotorsBehavior;
    type Network: RobotNetworkBehavior;
    type Peripherals: RobotPeripheralsBehavior;
}

#[derive(Copy, Clone)]
pub struct EmbassyInstant(Instant);

impl CrossPlatformInstant for EmbassyInstant {
    fn elapsed(&self) -> core::time::Duration {
        Instant::elapsed(&self.0).into()
    }

    fn checked_duration_since(&self, other: Self) -> Option<core::time::Duration> {
        Instant::checked_duration_since(&self.0, other.0).map(|x| x.into())
    }
}

impl Default for EmbassyInstant {
    fn default() -> Self {
        Self(Instant::now())
    }
}
