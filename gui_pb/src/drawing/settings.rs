use crate::App;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::settings::{
    ConnectionSettings, CvLocationSource, ShouldDoTargetPath, StrategyChoice,
};
use core_pb::messages::{
    GameServerCommand, GuiToServerMessage, NetworkStatus, ServerToRobotMessage,
    ServerToSimulationMessage,
};
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use core_pb::threaded_websocket::TextOrT;
use eframe::egui;
use eframe::egui::{Align, Color32, Layout, TextEdit, Ui, WidgetText};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

pub struct UiSettings {
    pub selected_robot: RobotName,
    pub any_robot_has_been_selected: bool,

    pub mdrc_server: ConnectionSettings,

    pub mdrc_server_collapsed: bool,
    pub simulation_collapsed: bool,
    pub game_server_collapsed: bool,
    pub robots_collapsed: [bool; NUM_ROBOT_NAMES],
    pub graph_lines: [[bool; 4]; 3],

    pub record_motor_data: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            selected_robot: RobotName::Pierre,
            any_robot_has_been_selected: false,

            mdrc_server: ConnectionSettings {
                connect: true,
                ipv4: [127, 0, 0, 1],
                port: GUI_LISTENER_PORT,
            },

            mdrc_server_collapsed: true,
            simulation_collapsed: true,
            game_server_collapsed: true,
            robots_collapsed: [true; NUM_ROBOT_NAMES],
            graph_lines: [[true; 4]; 3],

            record_motor_data: false,
        }
    }
}

fn validated<T: PartialEq>(
    id: String,
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut T,
    text: impl Into<WidgetText>,
    validation: fn(&str) -> Option<T>,
    to_str: fn(&T) -> String,
) {
    let text = text.into();

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.label(text);

        // if this is the first time seeing this field, set its string to the given value
        if !fields.contains_key(&id) {
            let str = to_str(value);
            fields.insert(id.clone(), (str.clone(), str));
        }
        let (last_typed, last_valid) = fields.get_mut(&id).unwrap();

        let field = ui.add(TextEdit::singleline(last_typed).desired_width(80.0));
        if let Some(t) = validation(last_typed.to_string().as_str()) {
            *last_valid = last_typed.to_string();
            // if they're not in the text box, and a new value has come in, replace it
            if !field.has_focus() && t != *value {
                let str = to_str(value);
                last_valid.clone_from(&str);
                *last_valid = str;
            } else {
                *value = t;
            }
        } else if !field.has_focus() {
            // if they're not in the text box, and they typed something invalid, just go back
            last_typed.clone_from(last_valid);
        }
    });
    ui.end_row();
}

pub fn num<T: FromStr + ToString + PartialEq>(
    id: String,
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut T,
    text: impl Into<WidgetText>,
) {
    validated(
        id,
        ui,
        fields,
        value,
        text,
        |x| x.parse().ok(),
        T::to_string,
    )
}

fn ipv4(
    id: String,
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut [u8; 4],
    text: impl Into<WidgetText>,
) {
    validated(
        id,
        ui,
        fields,
        value,
        text,
        |x| {
            // should be like xxx.xxx.xxx.xxx
            let re = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
            if re.is_match(x) {
                let mut arr = [0; 4];
                for (i, s) in x.split('.').enumerate() {
                    arr[i] = s.parse().unwrap();
                }
                Some(arr)
            } else {
                None
            }
        },
        |x| format!("{}.{}.{}.{}", x[0], x[1], x[2], x[3]),
    )
}

pub fn dropdown<T: Debug + PartialEq + Clone>(
    ui: &mut Ui,
    id: String,
    text: impl Into<WidgetText>,
    value: &mut T,
    options: &[T],
) {
    let s_text = WidgetText::from(format!("{:?}", value));
    egui::ComboBox::new(id, text)
        .selected_text(s_text)
        .show_ui(ui, |ui| {
            for t in options {
                let str = WidgetText::from(format!("{:?}", t));
                ui.selectable_value(value, t.clone(), str);
            }
        });
}

fn collapsable_section(
    ui: &mut Ui,
    collapsed: &mut bool,
    button_color: Color32,
    header_contents: impl FnOnce(&mut Ui),
    body_contents: impl FnOnce(&mut Ui),
    tooltip: Option<impl Into<WidgetText>>,
) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let mut button = ui.add(
            egui::Button::new(match *collapsed {
                true => egui_phosphor::regular::CARET_RIGHT,
                false => egui_phosphor::regular::CARET_DOWN,
            })
            .fill(button_color),
        );
        if let Some(tt) = tooltip {
            button = button.on_hover_text(tt)
        }
        if button.clicked() {
            *collapsed = !*collapsed;
        }
        header_contents(ui);
    });
    ui.end_row();
    if !*collapsed {
        body_contents(ui);
        ui.label("");
        ui.end_row();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generic_server(
    ui: &mut Ui,
    name: &str,
    fields: &mut HashMap<String, (String, String)>,
    connection_settings: &mut ConnectionSettings,
    collapsed: &mut bool,
    status: &NetworkStatus,
    right_ui: impl FnOnce(&mut Ui),
    tooltip: Option<impl Into<WidgetText>>,
) {
    let ip_name = name.to_string() + "server_ip";
    let port_name = name.to_string() + "server_port";
    collapsable_section(
        ui,
        collapsed,
        status.status().to_color32(),
        |ui| {
            ui.checkbox(&mut connection_settings.connect, name);
            ui.with_layout(Layout::right_to_left(Align::Center), right_ui);
        },
        |ui| {
            ipv4(ip_name, ui, fields, &mut connection_settings.ipv4, "IP");
            num(port_name, ui, fields, &mut connection_settings.port, "Port");
        },
        tooltip,
    );
}

pub fn draw_settings(app: &mut App, ui: &mut Ui) {
    let mut fields = app.settings_fields.take().unwrap();

    egui::Grid::new("settings_grid")
        .num_columns(1)
        .striped(true)
        .show(ui, |ui| draw_settings_inner(app, ui, &mut fields));

    app.settings_fields = Some(fields);
}

/// Reduce indentation
fn draw_settings_inner(app: &mut App, ui: &mut Ui, fields: &mut HashMap<String, (String, String)>) {
    ui.checkbox(&mut app.rotated_grid, "Rotated grid");
    ui.end_row();
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.checkbox(&mut app.settings.simulation.simulate, "Run simulation");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(
                    app.settings.simulation.simulate,
                    egui::Button::new(egui_phosphor::regular::ARROW_CLOCKWISE),
                )
                .clicked()
            {
                app.send(GuiToServerMessage::RestartSimulation);
            }
        });
    });
    ui.end_row();
    ui.checkbox(&mut app.settings.safe_mode, "Safe mode");
    ui.end_row();
    dropdown(
        ui,
        "Target Path Dropdown".to_string(),
        "Target Path",
        &mut app.settings.do_target_path,
        &ShouldDoTargetPath::get_all(),
    );
    ui.end_row();
    num(
        "target_speed".to_string(),
        ui,
        fields,
        &mut app.settings.target_speed,
        "Target speed",
    );
    ui.add_enabled_ui(
        app.server_status.robots[app.ui_settings.selected_robot as usize].connection
            == NetworkStatus::Connected,
        |ui| {
            if ui.button("Reset angle").clicked() {
                app.send(GuiToServerMessage::RobotCommand(
                    app.ui_settings.selected_robot,
                    ServerToRobotMessage::ResetAngle,
                ));
            }
        },
    );
    ui.end_row();
    ui.end_row();

    ui.separator();
    ui.end_row();

    dropdown(
        ui,
        "selected_robot".to_string(),
        "Selected",
        &mut app.ui_settings.selected_robot,
        &RobotName::get_all(),
    );
    ui.end_row();

    dropdown(
        ui,
        "gs_robot".to_string(),
        "Pacman",
        &mut app.settings.pacman,
        &RobotName::get_all()
            .into_iter()
            .filter(|name| name.is_simulated())
            .collect::<Vec<_>>(),
    );
    ui.end_row();

    ui.separator();
    ui.end_row();

    generic_server(
        ui,
        "MDRC Server",
        fields,
        &mut app.ui_settings.mdrc_server,
        &mut app.ui_settings.mdrc_server_collapsed,
        &app.network.0.status(),
        |_| {},
        None::<&str>,
    );

    generic_server(
        ui,
        "Simulation",
        fields,
        &mut app.settings.simulation.connection,
        &mut app.ui_settings.simulation_collapsed,
        &app.server_status.simulation_connection,
        |_| {},
        None::<&str>,
    );

    generic_server(
        ui,
        if app.server_status.advanced_game_server {
            "Game server++"
        } else {
            "Game server"
        },
        fields,
        &mut app.settings.game_server.connection,
        &mut app.ui_settings.game_server_collapsed,
        &app.server_status.game_server_connection,
        |ui| {
            let connected = app.server_status.game_server_connection == NetworkStatus::Connected;
            let reset_button = ui.add_enabled(
                connected,
                egui::Button::new(egui_phosphor::regular::ARROW_CLOCKWISE),
            );
            if reset_button.clicked() {
                app.network
                    .0
                    .send(TextOrT::T(GuiToServerMessage::GameServerCommand(
                        GameServerCommand::Reset,
                    )));
            }
            let icon = if app.server_status.game_state.paused {
                egui_phosphor::regular::PLAY
            } else {
                egui_phosphor::regular::PAUSE
            };
            let pause_button = ui.add_enabled(connected, egui::Button::new(icon));
            if pause_button.clicked() {
                app.network
                    .0
                    .send(TextOrT::T(GuiToServerMessage::GameServerCommand(
                        if app.server_status.game_state.paused {
                            GameServerCommand::Unpause
                        } else {
                            GameServerCommand::Pause
                        },
                    )));
            }
        },
        None::<&str>,
    );

    ui.separator();
    ui.end_row();

    let mut any_robot_enabled = None;
    for name in RobotName::get_all() {
        let conn_status = app.server_status.robots[name as usize].connection;
        generic_server(
            ui,
            &format!("{name}"),
            fields,
            &mut app.settings.robots[name as usize].connection,
            &mut app.ui_settings.robots_collapsed[name as usize],
            &app.server_status.robots[name as usize].connection,
            |ui| {
                ui.add_enabled_ui(conn_status == NetworkStatus::Connected, |ui| {
                    if ui.button(egui_phosphor::regular::ARROW_CLOCKWISE).clicked() {
                        app.network
                            .0
                            .send(TextOrT::T(GuiToServerMessage::RobotCommand(
                                name,
                                ServerToRobotMessage::Reboot,
                            )));
                    }
                });
                if name.is_simulated() {
                    ui.add_enabled_ui(
                        app.server_status.simulation_connection == NetworkStatus::Connected,
                        |ui| {
                            if app.server_status.robots[name as usize]
                                .sim_position
                                .is_some()
                            {
                                if ui.button(egui_phosphor::regular::MINUS).clicked() {
                                    app.network.0.send(TextOrT::T(
                                        GuiToServerMessage::SimulationCommand(
                                            ServerToSimulationMessage::Delete(name),
                                        ),
                                    ));
                                }
                            } else if ui.button(egui_phosphor::regular::PLUS).clicked() {
                                app.network.0.send(TextOrT::T(
                                    GuiToServerMessage::SimulationCommand(
                                        ServerToSimulationMessage::Spawn(name),
                                    ),
                                ));
                            }
                        },
                    );
                }
            },
            app.server_status.robots[name as usize]
                .ping
                .map(|x| format!("Ping: {x:?}")),
        );
        if app.settings.robots[name as usize].connection.connect {
            any_robot_enabled = Some(name);
        }
    }
    if let Some(name) = any_robot_enabled {
        if !app.ui_settings.any_robot_has_been_selected {
            app.ui_settings.selected_robot = name;
            app.ui_settings.any_robot_has_been_selected = true;
        }
    }

    ui.separator();
    ui.end_row();

    dropdown(
        ui,
        "strategy".to_string(),
        "Strategy",
        &mut app.settings.driving.strategy,
        &[
            StrategyChoice::Manual,
            StrategyChoice::ReinforcementLearning,
            StrategyChoice::TestUniform,
            StrategyChoice::TestForward,
        ],
    );
    ui.end_row();
    dropdown(
        ui,
        "cv_location".to_string(),
        "CV Location",
        &mut app.settings.cv_location_source,
        &[CvLocationSource::GameState, CvLocationSource::Localization],
    );
    ui.end_row();
}
