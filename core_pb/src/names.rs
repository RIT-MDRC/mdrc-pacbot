//! See [`RobotName`], a unique identifier for each known robot

/// The number of unique [`RobotName`]s
pub const NUM_ROBOT_NAMES: usize = 5;

/// Represents a unique robot, either a physical device or a simulation
///
/// Robot names are six letters, where the first letter indicates its type:
/// - names beginning with 'S' are simulated robots, not real
/// - names beginning with 'P' are raspberry pi pico boards
///
/// See [`NUM_ROBOT_NAMES`] for the number of names
///
/// usize values should be consecutive such that an array like `[(); NUM_ROBOT_NAMES]`
/// can be indexed like `arr[robot_name as usize]`
///
/// However, while they are set at compile time, these values are not stable over the
/// development of the codebase; code should not, for example, specifically rely on
/// [`Stella`] as index 0
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RobotName {
    // [P]ico boards
    Pierre = 0,
    Prince = 1,
    // Patric,
    // Pancho,
    // [S]imulated robots
    Stella = 2,
    Stevie = 3,
    Speers = 4,
}

impl From<usize> for RobotName {
    fn from(value: usize) -> Self {
        match value {
            0 => Pierre,
            1 => Prince,
            2 => Stella,
            3 => Stevie,
            4 => Speers,
            _ => panic!("Invalid robot name index: {}", value),
        }
    }
}

impl Display for RobotName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stella => write!(f, "Stella"),
            Stevie => write!(f, "Stevie"),
            Speers => write!(f, "Speers"),
            Pierre => write!(f, "Pierre"),
            Prince => write!(f, "Prince"),
        }
    }
}

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use RobotName::*;

impl RobotName {
    /// All robot names in order
    pub fn get_all() -> [RobotName; NUM_ROBOT_NAMES] {
        [Pierre, Prince, Stella, Stevie, Speers]
    }

    /// Whether this robot is managed by the simulator
    pub fn is_simulated(&self) -> bool {
        self.mac_address()[0] == 0x02
    }

    /// Whether this robot is a raspberry pi pico
    pub fn is_pico(&self) -> bool {
        match self {
            Pierre | Prince => true,
            _ => false,
        }
    }

    /// The mac address of this robot, must be unique
    ///
    /// Simulated robots look like 02:00:00:00::00:xx
    pub fn mac_address(&self) -> [u16; 6] {
        match self {
            Stella => [0x02, 0, 0, 0, 0, 0x01],
            Stevie => [0x02, 0, 0, 0, 0, 0x02],
            Speers => [0x02, 0, 0, 0, 0, 0x03],

            Pierre => [0x28, 0xcd, 0xc1, 0x0f, 0x82, 0x87],
            Prince => [0x28, 0xcd, 0xc1, 0x0f, 0x82, 0x88],
        }
    }

    /// Uniquely determine the robot name from the mac address, if recognized
    ///
    /// Simulated robots look like 02:00:00:00::00:xx
    pub fn from_mac_address(address: &[u16; 6]) -> Option<Self> {
        match address {
            [0x02, 0x00, 0x00, 0x00, 0x00, x] => match x {
                0x01 => Some(Stella),
                0x02 => Some(Stevie),
                0x03 => Some(Speers),
                _ => None,
            },

            [0x28, 0xcd, 0xc1, 0x0f, 0x82, 0x87] => Some(Pierre),
            [0x28, 0xcd, 0xc1, 0x0f, 0x82, 0x88] => Some(Prince),

            _ => None,
        }
    }

    /// The default pre-filled ip - robots need not necessarily use this ip
    pub fn default_ip(&self) -> [u8; 4] {
        match self {
            Pierre => [192, 168, 0, 1],
            Prince => [192, 168, 0, 2],
            // simulated robots are local
            _ => [127, 0, 0, 1],
        }
    }

    /// The port this robot will listen on for TCP connections
    ///
    /// Physical robots may listen on the same port
    pub fn port(&self) -> u16 {
        match self {
            // simulated robots each require their own port
            // spaced out in case an additional port is desired for each in the future
            Stella => 20022,
            Stevie => 20024,
            Speers => 20026,

            // picos may share ports
            _ => 20020,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_robot_names() {
        for i in 0..NUM_ROBOT_NAMES {
            let name: RobotName = i.into();
            assert_eq!(
                name,
                RobotName::get_all()[i],
                "usize value should match index of get_all"
            );
            assert!(
                !(name.is_pico() && name.is_simulated()),
                "a robot cannot be both a pico and simulated"
            );
            assert_eq!(
                Some(name),
                RobotName::from_mac_address(&name.mac_address()),
                "mac_address() and from_mac_address() match"
            );
            if name.is_simulated() {
                assert_eq!(name.mac_address()[0], 0x02);
                assert_eq!(name.default_ip(), [127, 0, 0, 1]);
            } else {
                assert_ne!(name.mac_address()[0], 0x02);
            }
            for other in RobotName::get_all() {
                if name != other {
                    assert_ne!(
                        name.mac_address(),
                        other.mac_address(),
                        "robots cannot share the same mac address"
                    );
                    if name.is_simulated() && other.is_simulated() {
                        assert_ne!(
                            name.port(),
                            other.port(),
                            "simulated robots cannot share the same port"
                        )
                    }
                }
            }
        }
    }
}
