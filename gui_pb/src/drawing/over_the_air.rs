use eframe::egui;
use eframe::egui::{Color32, RichText, Ui};
use std::time::Duration;

use crate::App;
use core_pb::messages::ota::{OverTheAirStep, OverTheAirStepCompletion};
use core_pb::messages::{GuiToServerMessage, NetworkStatus};
use core_pb::threaded_websocket::TextOrT;

pub fn draw_over_the_air(app: &mut App, ui: &mut Ui) {
    let name = app.ui_settings.selected_robot;

    ui.label(format!("Upload code to: {name}"));

    if app.server_status.robots[name as usize].connection != NetworkStatus::Connected {
        ui.label(
            RichText::new(format!(
                "{} {name} is not connected",
                egui_phosphor::regular::WARNING
            ))
            .color(Color32::YELLOW),
        );
    }

    ui.separator();

    let current_status = app.server_status.robots[name as usize].ota_current;
    ui.horizontal(|ui| {
        let start_enabled = current_status == OverTheAirStep::GuiRequest
            || current_status == OverTheAirStep::Failed
            || current_status == OverTheAirStep::Finished;
        ui.add_enabled(start_enabled, |ui: &mut Ui| {
            let button = ui.button("Start");
            if button.clicked() {
                app.network
                    .0
                    .send(TextOrT::T(GuiToServerMessage::StartOtaFirmwareUpdate(name)));
            }
            button
        });
        ui.add_enabled(
            current_status == OverTheAirStep::GuiConfirmation
                || current_status == OverTheAirStep::FinalGuiConfirmation,
            |ui: &mut Ui| {
                let button = ui.button("Confirm");
                if button.clicked() {
                    app.network
                        .0
                        .send(TextOrT::T(GuiToServerMessage::ConfirmFirmwareUpdate(name)));
                }
                button
            },
        );
        ui.add_enabled(!start_enabled, |ui: &mut Ui| {
            let button = ui.button("Cancel");
            if button.clicked() {
                app.network
                    .0
                    .send(TextOrT::T(GuiToServerMessage::CancelOtaFirmwareUpdate(
                        name,
                    )));
            }
            button
        });
        if ui.button("Clear").clicked() {
            app.network
                .0
                .send(TextOrT::T(GuiToServerMessage::ClearFirmwareUpdateHistory(
                    name,
                )));
        }
    });

    ui.separator();

    let mut steps = app.server_status.robots[name as usize]
        .ota_completed
        .clone();
    let curr = app.server_status.robots[name as usize].ota_current;
    if steps.last().map(|x| x.step != curr).unwrap_or(true) {
        steps.push(OverTheAirStepCompletion {
            step: curr,
            since_beginning: Duration::from_secs(0),
            success: if curr == OverTheAirStep::Finished {
                Some(true)
            } else if curr == OverTheAirStep::Failed {
                Some(false)
            } else {
                None
            },
        });
    }

    egui::Grid::new("ota_grid").show(ui, |ui| {
        for OverTheAirStepCompletion { step, success, .. } in steps {
            let color = match (step, success) {
                (_, Some(true)) => Color32::GREEN,
                (_, Some(false)) => Color32::RED,
                (_, None) => Color32::YELLOW,
            };
            if success.is_none()
                && step != OverTheAirStep::GuiRequest
                && step != OverTheAirStep::GuiConfirmation
                && step != OverTheAirStep::FinalGuiConfirmation
            {
                ui.spinner();
            } else {
                ui.label(
                    RichText::new(match (step, success) {
                        (_, Some(true)) => egui_phosphor::regular::CHECK,
                        (_, Some(false)) => egui_phosphor::regular::X,
                        (_, None) => egui_phosphor::regular::WARNING,
                    })
                    .color(color),
                );
            }
            ui.label(step.message());
            ui.end_row();

            if success == Some(false) || step == OverTheAirStep::Finished {
                break;
            }
        }
    });
}
