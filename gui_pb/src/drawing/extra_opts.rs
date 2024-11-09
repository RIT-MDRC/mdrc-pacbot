use crate::drawing::settings::num;
use crate::App;
use core_pb::messages::ExtraOptsTypes;
use eframe::egui;
use eframe::egui::{Align, Layout, Ui};
use std::collections::HashMap;

pub fn draw_extra_opts(app: &mut App, ui: &mut Ui) {
    ui.heading("Extra Opts (for temporary testing only)");

    ui.checkbox(
        &mut app.settings.robots[app.ui_settings.selected_robot as usize].extra_opts_enabled,
        "Extra opts enabled",
    );

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("Set options here");
            draw_one_opts_set(
                ui,
                &mut app.settings.robots[app.ui_settings.selected_robot as usize].extra_opts,
                &mut app.settings_fields.as_mut().unwrap(),
                0,
            );
        });
        ui.vertical(|ui| {
            ui.label("Robot received options");
            draw_disabled_opts_set(
                ui,
                &mut app.server_status.robots[app.ui_settings.selected_robot as usize]
                    .received_extra_opts
                    .as_mut()
                    .unwrap_or(&mut ExtraOptsTypes::default()),
                1,
            );
        })
    });
}

fn draw_one_opts_set(
    ui: &mut Ui,
    opts: &mut ExtraOptsTypes,
    fields: &mut HashMap<String, (String, String)>,
    unique_id: usize,
) {
    egui::Grid::new(&format!("opts grid {unique_id}"))
        .num_columns(1)
        .striped(true)
        .show(ui, |ui| {
            for (i, b) in opts.opts_bool.iter_mut().enumerate() {
                ui.checkbox(b, format!("Boolean {i}"));
                ui.end_row();
            }
            for (i, n) in opts.opts_f32.iter_mut().enumerate() {
                num(
                    format!("opts {unique_id} f32 {i}"),
                    ui,
                    fields,
                    n,
                    format!("f32 {i}"),
                )
            }
            for (i, n) in opts.opts_i8.iter_mut().enumerate() {
                num(
                    format!("opts {unique_id} i8 {i}"),
                    ui,
                    fields,
                    n,
                    format!("i8 {i}"),
                )
            }
            for (i, n) in opts.opts_i32.iter_mut().enumerate() {
                num(
                    format!("opts {unique_id} i32 {i}"),
                    ui,
                    fields,
                    n,
                    format!("i32 {i}"),
                )
            }
        });
}

fn draw_disabled_opts_set(ui: &mut Ui, opts: &mut ExtraOptsTypes, unique_id: usize) {
    egui::Grid::new(format!("opts grid {unique_id}"))
        .num_columns(1)
        .striped(true)
        .show(ui, |ui| {
            for b in opts.opts_bool.iter_mut() {
                ui.add_enabled_ui(false, |ui| ui.checkbox(b, ""));
                ui.end_row();
            }
            for n in opts.opts_f32.iter_mut() {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(format!("{n:?}"));
                });
                ui.end_row();
            }
            for n in opts.opts_i8.iter_mut() {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(format!("{n:?}"));
                });
                ui.end_row();
            }
            for n in opts.opts_i32.iter_mut() {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(format!("{n:?}"));
                });
                ui.end_row();
            }
        });
}
