use crate::ota::OverTheAirProgramming;
use crate::sockets::Destination::*;
use crate::sockets::Incoming::*;
use crate::sockets::Outgoing::*;
use crate::sockets::{Incoming, Outgoing, Sockets};
use crate::App;
use core_pb::bin_encode;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::settings::PacbotSettings;
use core_pb::messages::{
    GuiToServerMessage, NetworkStatus, RobotToServerMessage, ServerToGuiMessage,
    ServerToRobotMessage, GAME_SERVER_MAGIC_NUMBER,
};
use core_pb::names::RobotName;
use core_pb::pacbot_rs::game_state::GameState;
use core_pb::util::utilization::UtilizationMonitor;
use std::time::{Duration, Instant};

pub async fn manage_network() {
    let sockets = Sockets::spawn().await;

    let mut app = App {
        status: Default::default(),
        settings: Default::default(),
        utilization_monitor: UtilizationMonitor::default(),

        settings_update_needed: false,

        client_http_host_process: None,
        sim_game_engine_process: None,

        over_the_air_programming: OverTheAirProgramming::new(sockets.outgoing.clone()),

        sockets,

        grid: Default::default(),
    };

    app.utilization_monitor.start();

    println!("Listening on 0.0.0.0:{GUI_LISTENER_PORT}");

    // apply default settings to the app
    app.update_settings(&PacbotSettings::default(), PacbotSettings::default())
        .await;

    let mut previous_200ms_tick = Instant::now();

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

        // frequently send motor commands to robots
        if previous_200ms_tick.elapsed() > Duration::from_millis(200) {
            previous_200ms_tick = Instant::now();
            for name in RobotName::get_all() {
                let id = name as usize;
                // pwm overrides
                if app.settings.robots[id]
                    .pwm_override
                    .iter()
                    .any(|x| x[0].is_some() || x[1].is_some())
                {
                    app.send(
                        Robot(name),
                        ToRobot(ServerToRobotMessage::PwmOverride(
                            app.settings.robots[id].pwm_override,
                        )),
                    )
                    .await;
                }
                // motor overrides
                if app.settings.robots[id]
                    .set_point_override
                    .iter()
                    .any(|x| x.is_some())
                {
                    app.send(
                        Robot(name),
                        ToRobot(ServerToRobotMessage::MotorsOverride(
                            app.settings.robots[id].set_point_override,
                        )),
                    )
                    .await;
                }
            }
        }

        app.over_the_air_programming.tick(&mut app.status).await;

        // we want to measure the amount of time the server spends processing messages,
        // which shouldn't include the amount of time spent waiting for messages
        app.utilization_monitor.stop();
        app.status.utilization = app.utilization_monitor.status();
        let msg = app.sockets.incoming.recv().await.unwrap();
        app.utilization_monitor.start();

        if app.settings.safe_mode {
            if let FromRobot(msg) = &msg.1 {
                let encoded = bin_encode(msg.clone()).unwrap();
                if encoded[0] > 7 {
                    continue;
                }
            }
        }
        app.over_the_air_programming
            .update(&msg, &mut app.status)
            .await;
        match msg {
            (dest, Bytes(data)) => eprintln!(
                "Unexpectedly received {} raw bytes from {dest:?}",
                data.len()
            ),
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
            (Robot(name), FromRobot(RobotToServerMessage::Name(_))) => {
                println!("Received name from {name}");
                app.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::MotorConfig(
                        app.settings.robots[name as usize].motor_config,
                    )),
                )
                .await;
                app.send(
                    Robot(name),
                    ToRobot(ServerToRobotMessage::Pid(
                        app.settings.robots[name as usize].pid,
                    )),
                )
                .await;
            }
            (Robot(name), FromRobot(RobotToServerMessage::MotorControlStatus(status))) => {
                app.status.robots[name as usize].last_motor_status = status;
            }
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
                GuiToServerMessage::RobotVelocity(_robot, _vel) => {
                    // let (lin, ang) = vel.unwrap_or((Vector2::zeros(), 0.0));
                    // println!(
                    //     "sending vel {lin:?} {ang:?} = {:?} to robot..",
                    //     RobotDefinition::default()
                    //         .drive_system
                    //         .get_motor_speed_omni(lin, ang)
                    // );
                    // app.send(
                    //     Robot(robot),
                    //     ToRobot(ServerToRobotMessage::TargetVelocity(lin, ang)),
                    // )
                    // .await
                }
                _ => {}
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
