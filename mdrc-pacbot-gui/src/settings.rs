use crate::AppData;
use eframe::egui;
use eframe::egui::{Align, Layout, Ui, WidgetText};
use mdrc_pacbot_server::messages::settings::StrategyChoice;
use regex::Regex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

fn validated<T: PartialEq>(
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut T,
    text: impl Into<WidgetText>,
    validation: fn(&str) -> Option<T>,
    to_str: fn(&T) -> String,
) {
    let text = text.into();
    let text_str = text.text().to_string();

    // if this is the first time seeing this field, set its string to the given value
    if !fields.contains_key(&text_str) {
        let str = to_str(value);
        fields.insert(text_str.clone(), (str.clone(), str));
    }
    let (last_typed, last_valid) = fields.get_mut(&text_str).unwrap();

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.label(text);
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
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut T,
    text: impl Into<WidgetText>,
) {
    validated(ui, fields, value, text, |x| x.parse().ok(), T::to_string)
}

fn ip(
    ui: &mut Ui,
    fields: &mut HashMap<String, (String, String)>,
    value: &mut String,
    text: impl Into<WidgetText>,
) {
    validated(
        ui,
        fields,
        value,
        text,
        |x| {
            // should be like xxx.xxx.xxx.xxx:xxxx
            let re = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d{1,5}$").unwrap();
            if re.is_match(x) {
                Some(x.to_string())
            } else {
                None
            }
        },
        String::to_string,
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

pub fn draw_settings(app: &mut AppData, ui: &mut Ui) {
    let mut fields = app.settings_fields.take().unwrap();

    egui::Grid::new("settings_grid")
        .num_columns(1)
        .striped(true)
        .show(ui, |ui| {
            ui.checkbox(&mut true, "Scan for server");
            ui.end_row();
            ip(ui, &mut fields, &mut app.settings.pico.ip, "IP");

            ui.checkbox(&mut app.rotated_grid, "Rotated grid");
            ui.end_row();
            ui.label("");
            ui.end_row();

            num(
                ui,
                &mut fields,
                &mut app.settings.particle_filter.pf_cv_error_std,
                "Cv error std",
            );
            num(
                ui,
                &mut fields,
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
        });

    app.settings_fields = Some(fields);
}
