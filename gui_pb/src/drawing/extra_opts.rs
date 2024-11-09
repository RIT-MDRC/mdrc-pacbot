use crate::drawing::settings::num;
use crate::App;
use core_pb::messages::ExtraOptsTypes;
use eframe::egui;
use eframe::egui::Ui;

pub fn draw_extra_opts(app: &mut App, ui: &mut Ui) {
    ui.heading("Extra Opts (for temporary testing only)");

    ui.checkbox(
        &mut app.settings.robots[app.ui_settings.selected_robot as usize].extra_opts_enabled,
        "Extra opts enabled",
    );

    let set = &mut app.settings.robots[app.ui_settings.selected_robot as usize].extra_opts;
    let mut def = ExtraOptsTypes::default();
    let mut def2 = ExtraOptsTypes::default();
    let rcv = app.server_status.robots[app.ui_settings.selected_robot as usize]
        .received_extra_opts
        .as_mut()
        .unwrap_or(&mut def);
    let ind = app.server_status.robots[app.ui_settings.selected_robot as usize]
        .extra_indicators
        .as_mut()
        .unwrap_or(&mut def2);
    let fields = app.settings_fields.as_mut().unwrap();

    egui::Grid::new("opts grid")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Set options here");
            ui.label("Robot received options");
            ui.label("Robot extra indicators");
            ui.end_row();
            for i in 0..set.opts_bool.len() {
                ui.checkbox(&mut set.opts_bool[i], format!("Boolean {i}"));
                ui.add_enabled_ui(false, |ui| ui.checkbox(&mut rcv.opts_bool[i], ""));
                ui.add_enabled_ui(false, |ui| ui.checkbox(&mut ind.opts_bool[i], ""));
                ui.end_row();
            }
            ui.label("");
            ui.label("");
            ui.label("");
            ui.end_row();
            for i in 0..set.opts_f32.len() {
                num(
                    format!("opts f32 {i}"),
                    ui,
                    fields,
                    &mut set.opts_f32[i],
                    format!("f32 {i}"),
                    false,
                );
                ui.label(format!("{:?}", rcv.opts_f32[i]));
                ui.label(format!("{:?}", ind.opts_f32[i]));
                ui.end_row();
            }
            ui.label("");
            ui.label("");
            ui.label("");
            ui.end_row();
            for i in 0..set.opts_i8.len() {
                num(
                    format!("opts i8 {i}"),
                    ui,
                    fields,
                    &mut set.opts_i8[i],
                    format!("i8 {i}"),
                    false,
                );
                ui.label(format!("{:?}", rcv.opts_i8[i]));
                ui.label(format!("{:?}", ind.opts_i8[i]));
                ui.end_row();
            }
            ui.label("");
            ui.label("");
            ui.label("");
            ui.end_row();
            for i in 0..set.opts_i32.len() {
                num(
                    format!("opts i32 {i}"),
                    ui,
                    fields,
                    &mut set.opts_i32[i],
                    format!("i32 {i}"),
                    false,
                );
                ui.label(format!("{:?}", rcv.opts_i32[i]));
                ui.label(format!("{:?}", ind.opts_i32[i]));
                ui.end_row();
            }
        });
}
