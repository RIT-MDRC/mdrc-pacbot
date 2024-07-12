use crate::network::Sockets;
use core_pb::grid::computed_grid::ComputedGrid;
use core_pb::messages::server_status::ServerStatus;
use core_pb::messages::settings::PacbotSettings;
use core_pb::messages::GuiToGameServerMessage;
use futures_util::StreamExt;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::sleep;

pub mod network;
// todo pub mod strategy;

#[allow(dead_code)]
pub struct App {
    status: ServerStatus,
    send_updated_status: UnboundedSender<()>,

    client_http_host_process: Option<Child>,
    sim_game_engine_process: Option<Child>,

    grid: ComputedGrid,
}

#[tokio::main]
async fn main() {
    println!("RIT Pacbot server starting up");

    let (updated_status_tx, mut updated_status_rx) = unbounded_channel();

    let app = Arc::new(Mutex::new(App {
        send_updated_status: updated_status_tx,
        status: Default::default(),
        client_http_host_process: None,
        sim_game_engine_process: None,
        grid: Default::default(),
    }));

    let default_settings = PacbotSettings::default();

    let mut sockets = Sockets::spawn(app.clone());

    app.lock()
        .unwrap()
        .update_settings(&mut sockets, &default_settings, &default_settings)
        .await;

    loop {
        select! {
            _ = updated_status_rx.recv() => {
                sockets.gui_outgoing.unbounded_send(app.lock().unwrap().status.clone()).unwrap()
            }
            m = sockets.game_states.next() => {
                status(&app, |s| s.game_state = m.unwrap())
            },
            m = sockets.commands_from_gui.next() => {
                match m.unwrap() {
                    GuiToGameServerMessage::Settings(settings) => {
                        let mut app = app.lock().unwrap();
                        let old_settings = app.status.settings.clone();
                        app.update_settings(&mut sockets, &old_settings, &settings).await;
                        app.change_status(|s| s.settings = settings);
                    }
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                // gui clients expect a status once in a while
                status(&app, |_| {})
            }
        }
    }
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
        let old_settings = self.status.settings.clone();
        changes(&mut self.status);
        if old_settings != self.status.settings {
            self.status.settings.version += 1;
        }
        self.send_updated_status.send(()).unwrap()
    }

    async fn update_settings(
        &mut self,
        sockets: &mut Sockets,
        old: &PacbotSettings,
        new: &PacbotSettings,
    ) {
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
                sockets
                    .game_server_addr
                    .unbounded_send(Some((new.game_server.ipv4, new.game_server.ws_port)))
                    .unwrap();
            } else {
                sockets.game_server_addr.unbounded_send(None).unwrap();
            }
        }

        if new.simulate {
            if self.sim_game_engine_process.is_none() {
                self.sim_game_engine_process = Some(
                    Command::new("cargo")
                        .args(["run", "--bin", "sim_pb", "--release"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.sim_game_engine_process.take() {
                child.kill().unwrap();
            }
        }

        if new.host_http {
            if self.client_http_host_process.is_none() {
                self.client_http_host_process = Some(
                    Command::new("trunk")
                        .args(["serve", "--config", "gui_pb/Trunk.toml"])
                        .spawn()
                        .unwrap(),
                );
            }
        } else {
            if let Some(mut child) = self.client_http_host_process.take() {
                child.kill().unwrap();
            }
        }
    }
}
