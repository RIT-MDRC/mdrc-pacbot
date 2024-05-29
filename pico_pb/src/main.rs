#![no_std]
#![no_main]

mod i2c;
mod vl53l1x;
mod vl6180x;

use cortex_m_rt::entry;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    // println!("Hello, world!");
    loop {}
}
