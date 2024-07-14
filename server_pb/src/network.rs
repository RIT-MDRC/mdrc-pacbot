use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::settings::PacbotSettings;
use core_pb::messages::{
    GuiToServerMessage, NetworkStatus, ServerToGuiMessage, ServerToSimulationMessage,
    GAME_SERVER_MAGIC_NUMBER,
};
use core_pb::pacbot_rs::game_state::GameState;

use crate::sockets::Destination::*;
use crate::sockets::Incoming::*;
use crate::sockets::Outgoing::*;
use crate::sockets::{Incoming, Outgoing, Sockets};
use crate::App;

pub async fn manage_network() {
    let mut app = App {
        status: Default::default(),
        settings: Default::default(),

        settings_update_needed: false,

        client_http_host_process: None,
        sim_game_engine_process: None,

        sockets: Sockets::spawn(),

        grid: Default::default(),
    };

    println!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");

    // apply default settings to the app
    app.update_settings(&PacbotSettings::default(), PacbotSettings::default())
        .await;

    loop {
        // if necessary, send updated settings to clients
        if app.settings_update_needed {
            app.settings_update_needed = false;
            app.send(
                GuiClients,
                ToGui(ServerToGuiMessage::Settings(app.settings.clone())),
            )
            .await;
        }

        match app.sockets.incoming.recv().await.unwrap() {
            (_, SleepFinished) => {
                // send updated status to clients every so often
                app.send(
                    GuiClients,
                    ToGui(ServerToGuiMessage::Status(app.status.clone())),
                )
                .await
            }
            (dest, Status(status)) => match dest {
                Simulation => app.status.simulation_connection = status,
                Robot(name) => app.status.robots[name as usize].connection = status,
                GameServer => {
                    if status != NetworkStatus::Connected {
                        // assume the game server is not advanced until proven otherwise
                        app.status.advanced_game_server = false;
                    }
                    app.status.game_server_connection = status
                }
                _ => {}
            },
            (_, FromGameServer(bytes)) => {
                if bytes == GAME_SERVER_MAGIC_NUMBER.to_vec() {
                    app.status.advanced_game_server = true;
                } else {
                    let mut g = GameState::new();
                    match g.update(&bytes) {
                        Ok(()) => app.status.game_state = g,
                        Err(e) => eprintln!("Error updating game state: {e:?}"),
                    }
                }
            }
            (_, FromSimulation(msg)) => println!("Message from simulation: {msg:?}"),
            (Robot(name), FromRobot(msg)) => println!("Message received from {name}: {msg:?}"),
            (Robot(_), _) => {}
            (_, FromRobot(_)) => {}
            (_, FromGui(msg)) => match msg {
                GuiToServerMessage::Settings(settings) => {
                    let old_settings = app.settings.clone();
                    app.update_settings(&old_settings, settings).await;
                }
                GuiToServerMessage::GameServerCommand(command) => match command.text() {
                    Some(text) => app.send(GameServer, Outgoing::Text(text.into())).await,
                    None => {
                        if app.status.advanced_game_server {
                            app.send(GameServer, ToGameServer(command)).await;
                        }
                    }
                },
                GuiToServerMessage::RobotVelocity(robot, vel) => {
                    app.send(
                        Simulation,
                        ToSimulation(ServerToSimulationMessage::RobotVelocity(robot, vel)),
                    )
                    .await
                }
            },
            (_, GuiConnected(id)) => {
                app.status.gui_clients += 1;
                println!(
                    "Gui client #{id} connected; {} gui client(s) are connected",
                    app.status.gui_clients
                );
                app.settings_update_needed = true;
            }
            (_, GuiDisconnected(id)) => {
                app.status.gui_clients -= 1;
                println!(
                    "Gui client #{id} disconnected; {} gui client(s) remaining",
                    app.status.gui_clients
                );
                app.settings_update_needed = true;
            }
            (dest, Incoming::Text(text)) => eprintln!("Unexpected text from {dest:?}: {text}"),
        }
    }
}
