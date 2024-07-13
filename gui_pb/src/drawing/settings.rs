use crate::colors::network_status_to_color;
use crate::App;
use core_pb::constants::GUI_LISTENER_PORT;
use core_pb::messages::settings::{ConnectionSettings, StrategyChoice};
use core_pb::messages::NetworkStatus;
use core_pb::names::{RobotName, NUM_ROBOT_NAMES};
use eframe::egui;
use eframe::egui::{Align, Color32, Layout, Ui, WidgetText};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

pub struct UiSettings {
    pub selected_robot: RobotName,

    pub mdrc_server: ConnectionSettings,

    pub mdrc_server_collapsed: bool,
    pub simulation_collapsed: bool,
    pub game_server_collapsed: bool,
    pub robots_collapsed: [bool; NUM_ROBOT_NAMES],
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            selected_robot: RobotName::Stella,

            mdrc_server: ConnectionSettings {
                connect: true,
                ipv4: [127, 0, 0, 1],
                port: GUI_LISTENER_PORT,
            },

            mdrc_server_collapsed: true,
            simulation_collapsed: true,
            game_server_collapsed: true,
            robots_collapsed: [true; 5],
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

        let field = ui.text_edit_singleline(last_typed);
        if let Some(t) = validation(last_typed.to_string().as_str()) {
            *last_valid = last_typed.to_string();
            // if they're not in the text box, and a new value has come in, replace it
            if !field.has_focus() && t != *value {
                let str = to_str(value);
                *last_typed = str.clone();
                *last_valid = str;
            } else {
                *value = t;
            }
        } else if !field.has_focus() {
            // if they're not in the text box, and they typed something invalid, just go back
            *last_typed = last_valid.clone();
        }
    });
    ui.end_row();
}

fn num<T: FromStr + ToString + PartialEq>(
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
                let mut i = 0;
                for s in x.split('.') {
                    arr[i] = s.parse().unwrap();
                    i += 1;
                }
                Some(arr)
            } else {
                None
            }
        },
        |x| format!("{}.{}.{}.{}", x[0], x[1], x[2], x[3]),
    )
}

fn dropdown<T: Debug + PartialEq + Clone>(
    ui: &mut Ui,
    id: &str,
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
    ui.end_row();
}

fn collapsable_section(
    ui: &mut Ui,
    collapsed: &mut bool,
    button_color: Color32,
    header_contents: impl FnOnce(&mut Ui),
    body_contents: impl FnOnce(&mut Ui),
) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let button = ui.add(
            egui::Button::new(format!(
                "{}",
                match *collapsed {
                    true => egui_phosphor::regular::CARET_RIGHT,
                    false => egui_phosphor::regular::CARET_DOWN,
                }
            ))
            .fill(button_color),
        );
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

pub fn generic_server(
    ui: &mut Ui,
    name: &str,
    fields: &mut HashMap<String, (String, String)>,
    connection_settings: &mut ConnectionSettings,
    collapsed: &mut bool,
    status: &NetworkStatus,
) {
    let ip_name = name.to_string() + "server_ip";
    let port_name = name.to_string() + "server_port";
    collapsable_section(
        ui,
        collapsed,
        network_status_to_color(&status),
        |ui| {
            ui.checkbox(&mut connection_settings.connect, name);
        },
        |ui| {
            ipv4(ip_name, ui, fields, &mut connection_settings.ipv4, "IP");
            num(port_name, ui, fields, &mut connection_settings.port, "Port");
        },
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
    ui.checkbox(&mut app.settings.simulation.simulate, "Run simulation");
    ui.end_row();

    ui.separator();
    ui.end_row();

    dropdown(
        ui,
        "selected_robot",
        "Selected",
        &mut app.ui_settings.selected_robot,
        &RobotName::get_all(),
    );

    if app.settings.simulation.connection.connect {
        dropdown(
            ui,
            "gs_robot",
            "Pacman",
            &mut app.settings.simulation.pacman,
            &RobotName::get_all()
                .into_iter()
                .filter(|name| name.is_simulated())
                .collect::<Vec<_>>(),
        );
    }

    ui.separator();
    ui.end_row();

    generic_server(
        ui,
        "MDRC Server",
        fields,
        &mut app.ui_settings.mdrc_server,
        &mut app.ui_settings.mdrc_server_collapsed,
        &app.network.0.status(),
    );

    generic_server(
        ui,
        "Simulation",
        fields,
        &mut app.settings.simulation.connection,
        &mut app.ui_settings.simulation_collapsed,
        &app.server_status.simulation_connection,
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
    );

    ui.separator();
    ui.end_row();

    for name in RobotName::get_all() {
        generic_server(
            ui,
            &format!("{name}"),
            fields,
            &mut app.settings.robots[name as usize].connection,
            &mut app.ui_settings.robots_collapsed[name as usize],
            &app.server_status.robots[name as usize].connection,
        );
    }

    ui.separator();
    ui.end_row();

    // collapsable_section(
    //     ui,
    //     &mut app.ui_settings.robot_collapsed,
    //     TRANSLUCENT_YELLOW_COLOR,
    //     |ui| {
    //         ui.checkbox(&mut app.settings.game_server.connect, "Robot");
    //     },
    //     |ui| {
    //         ipv4("robot_ip", ui, fields, &mut app.settings.robots.ipv4, "IP");
    //         num(
    //             "robot_tcp_port",
    //             ui,
    //             fields,
    //             &mut app.settings.robots.tcp_port,
    //             "TCP Port",
    //         );
    //     },
    // );

    dropdown(
        ui,
        "strategy",
        "Strategy",
        &mut app.settings.driving.strategy,
        &[
            StrategyChoice::Manual,
            StrategyChoice::TestUniform,
            StrategyChoice::TestForward,
        ],
    );
}
