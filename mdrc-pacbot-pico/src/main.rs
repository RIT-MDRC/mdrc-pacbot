#![no_std]
#![no_main]

use cortex_m_rt::entry;
#[allow(unused_imports)]
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    // println!("Hello, world!");
    loop {}
}
