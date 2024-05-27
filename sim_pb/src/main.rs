use crate::network::PacbotSimulation;
use std::thread::sleep;

mod network;

fn main() {
    let mut simulation = PacbotSimulation::new().unwrap();
    loop {
        if let Some(t) = simulation.time_to_update() {
            sleep(t)
        }
        simulation.update()
    }
}
