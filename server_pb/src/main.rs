use nalgebra::{Isometry2, Point2};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use crate::network::Sockets;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;

pub mod network;
pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,

    sockets: Sockets,

    sim_game_engine_thread: Option<Child>,

    grid: ComputedGrid,

    pacbot_location: Isometry2<f32>,
    // guaranteed to be a walkable cell
    pacbot_int_location: Point2<i8>,
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // let mut app = App::default();
    //
    // loop {
    //     reconnect_sockets(&mut app);
    //     sleep(Duration::from_millis(50));
    // }
}

fn status<F>(app: &Arc<Mutex<App>>, changes: F)
where
    F: FnOnce(&mut ServerStatus),
{
    app.lock().unwrap().change_status(changes)
}

impl App {
    pub fn change_status<F>(&mut self, changes: F)
    where
        F: FnOnce(&mut ServerStatus),
    {
        changes(&mut self.status);
        self.sockets
            .gui_outgoing
            .unbounded_send(self.status.clone())
            .unwrap()
    }

    fn update_settings(&mut self, old: &PacbotSettings, new: &PacbotSettings) {
        if (
            new.game_server.connect,
            new.game_server.ipv4,
            new.game_server.ws_port,
        ) != (
            old.game_server.connect,
            old.game_server.ipv4,
            old.game_server.ws_port,
        ) {
            if new.game_server.connect {
                // todo
            } else {
            }
        }

        if new.game_server.simulate {
            if self.sim_game_engine_thread.is_none() {
                self.sim_game_engine_thread = Some(
                    Command::new("cargo")
                        .args(["run", "--bin", "sim_pb"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.sim_game_engine_thread.take() {
                child.kill().unwrap();
            }
        }
    }
}
