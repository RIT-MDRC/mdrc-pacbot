use crate::colors::TRANSLUCENT_YELLOW_COLOR;
use crate::network::network_status_to_color;
use crate::AppData;
use core_pb::messages::settings::StrategyChoice;
use eframe::egui;
use eframe::egui::{Align, Color32, Layout, Ui, WidgetText};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

fn validated<T: PartialEq>(
    id: &'static str,
    ui: &mut Ui,
    fields: &mut HashMap<&str, (String, String)>,
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
            fields.insert(id, (str.clone(), str));
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
    id: &'static str,
    ui: &mut Ui,
    fields: &mut HashMap<&str, (String, String)>,
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
    id: &'static str,
    ui: &mut Ui,
    fields: &mut HashMap<&str, (String, String)>,
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

pub fn draw_settings(app: &mut AppData, ui: &mut Ui) {
    let mut fields = app.settings_fields.take().unwrap();

    egui::Grid::new("settings_grid")
        .num_columns(1)
        .striped(true)
        .show(ui, |ui| draw_settings_inner(app, ui, &mut fields));

    app.settings_fields = Some(fields);
}

/// Reduce indentation
fn draw_settings_inner(
    app: &mut AppData,
    ui: &mut Ui,
    fields: &mut HashMap<&str, (String, String)>,
) {
    ui.checkbox(&mut app.rotated_grid, "Rotated grid");
    ui.end_row();
    ui.checkbox(&mut app.settings.simulate, "Simulated Physics/Game Server");
    ui.end_row();

    collapsable_section(
        ui,
        &mut app.ui_settings.mdrc_server_collapsed,
        network_status_to_color(app.network_data.status()),
        |ui| {
            ui.checkbox(&mut app.ui_settings.connect_mdrc_server, "MDRC Server");
        },
        |ui| {
            ipv4(
                "server_ip",
                ui,
                fields,
                &mut app.ui_settings.mdrc_server_ipv4,
                "IP",
            );
            num(
                "server_port",
                ui,
                fields,
                &mut app.ui_settings.mdrc_server_ws_port,
                "Port",
            );
        },
    );

    collapsable_section(
        ui,
        &mut app.ui_settings.game_server_collapsed,
        TRANSLUCENT_YELLOW_COLOR,
        |ui| {
            ui.checkbox(&mut app.settings.game_server.connect, "Game server");
        },
        |ui| {
            ipv4(
                "game_server_ip",
                ui,
                fields,
                &mut app.settings.game_server.ipv4,
                "IP",
            );
            num(
                "game_server_port",
                ui,
                fields,
                &mut app.settings.game_server.ws_port,
                "Port",
            );
        },
    );

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

    num(
        "cv_err_std",
        ui,
        fields,
        &mut app.settings.particle_filter.pf_cv_error_std,
        "Cv error std",
    );
    num(
        "num_gui_pts",
        ui,
        fields,
        &mut app.settings.particle_filter.pf_gui_points,
        "Num gui points",
    );
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
