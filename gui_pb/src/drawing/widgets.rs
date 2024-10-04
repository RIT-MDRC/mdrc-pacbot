use crate::App;
use core_pb::messages::NetworkStatus;
use core_pb::util::ColoredStatus;
use eframe::egui;
use eframe::egui::{RichText, Ui};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum PacbotWidget {
    GridWidget,
    UtilizationWidget,
    SensorsWidget,
    BatteryWidget,
}

pub fn draw_widgets(app: &mut App, ui: &mut Ui) {
    for widget_name in [
        PacbotWidget::GridWidget,
        PacbotWidget::UtilizationWidget,
        PacbotWidget::SensorsWidget,
        PacbotWidget::BatteryWidget,
    ] {
        let button = ui.add(
            egui::Button::new(widget_name.button_text(app))
                .fill(widget_name.overall_status(app).to_color32()),
        );
        if widget_name != PacbotWidget::GridWidget {
            button.on_hover_ui(|ui| widget_name.hover_ui(app, ui));
        }
    }
}

fn draw_status(ui: &mut Ui, status: &ColoredStatus, label: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(status.icon()).color(status.to_color32_solid()));
        let label: String = label.into();
        if label != "" {
            ui.label(format!(
                "{}: {}",
                label,
                status.message().unwrap_or("?".to_string())
            ));
        } else {
            ui.label(format!("{}", status.message().unwrap_or("?".to_string())));
        }
    });
}

impl PacbotWidget {
    pub fn button_text(&self, app: &mut App) -> RichText {
        match self {
            PacbotWidget::GridWidget => RichText::new(format!(
                "{} {} {} {} {} {}",
                egui_phosphor::regular::HEART,
                app.server_status.game_state.curr_lives,
                egui_phosphor::regular::TROPHY,
                app.server_status.game_state.curr_score,
                egui_phosphor::regular::TIMER,
                app.server_status.game_state.curr_ticks
            )),
            PacbotWidget::UtilizationWidget => RichText::new(egui_phosphor::regular::TIMER),
            PacbotWidget::SensorsWidget => RichText::new(egui_phosphor::regular::HEADLIGHTS),
            PacbotWidget::BatteryWidget => {
                let battery =
                    app.server_status.robots[app.ui_settings.selected_robot as usize].battery;
                RichText::new(if battery > 0.75 {
                    egui_phosphor::regular::BATTERY_FULL
                } else if battery > 0.5 {
                    egui_phosphor::regular::BATTERY_HIGH
                } else if battery > 0.25 {
                    egui_phosphor::regular::BATTERY_MEDIUM
                } else if battery > 0.1 {
                    egui_phosphor::regular::BATTERY_LOW
                } else {
                    egui_phosphor::regular::BATTERY_EMPTY
                })
            }
        }
    }

    pub fn overall_status(&self, app: &mut App) -> ColoredStatus {
        match self {
            PacbotWidget::GridWidget => ColoredStatus::NotApplicable(None),
            PacbotWidget::UtilizationWidget => vec![
                app.gui_stopwatch.status(),
                app.server_status.utilization.clone(),
                app.server_status.inference_time.clone(),
            ]
            .into_iter()
            .max_by_key(|x| x.severity())
            .unwrap(),
            PacbotWidget::SensorsWidget => {
                let robot = &app.server_status.robots[app.ui_settings.selected_robot as usize];
                if robot.connection != NetworkStatus::Connected {
                    ColoredStatus::NotApplicable(None)
                } else {
                    if robot.imu_angle.is_err() || robot.distance_sensors.iter().any(|x| x.is_err())
                    {
                        ColoredStatus::Error(None)
                    } else {
                        ColoredStatus::Ok(None)
                    }
                }
            }
            PacbotWidget::BatteryWidget => {
                app.server_status.robots[app.ui_settings.selected_robot as usize].battery_status()
            }
        }
    }

    pub fn hover_ui(&self, app: &mut App, ui: &mut Ui) {
        #[allow(clippy::single_match)]
        match self {
            PacbotWidget::BatteryWidget => draw_status(
                ui,
                &app.server_status.robots[app.ui_settings.selected_robot as usize].battery_status(),
                "",
            ),
            PacbotWidget::GridWidget => {}
            PacbotWidget::UtilizationWidget => {
                let status = app.gui_stopwatch.status();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(status.icon()).color(status.to_color32_solid()));
                    ui.label(format!(
                        "Gui: {:.1}% | {:.0} fps | {}",
                        app.gui_stopwatch.utilization().utilization() * 100.0,
                        app.gui_stopwatch.utilization().hz(),
                        status.message().unwrap_or("?".to_string())
                    ));
                });
                draw_status(ui, &app.server_status.utilization, "Server");
                draw_status(ui, &app.server_status.inference_time, "Inference");
            }
            PacbotWidget::SensorsWidget => {
                let robot = &app.server_status.robots[app.ui_settings.selected_robot as usize];
                if robot.connection != NetworkStatus::Connected {
                    draw_status(
                        ui,
                        &ColoredStatus::Error(Some("Not connected".to_string())),
                        "",
                    );
                }
                let imu_status = match robot.imu_angle {
                    Ok(angle) => ColoredStatus::Ok(Some(format!("{angle:.3}"))),
                    Err(_) => ColoredStatus::Error(Some("ERR".to_string())),
                };
                draw_status(ui, &imu_status, "IMU");
                for i in 0..4 {
                    let dist_status = match robot.distance_sensors[i] {
                        Ok(Some(dist)) => ColoredStatus::Ok(Some(format!("{dist:.3}"))),
                        Ok(None) => ColoredStatus::Warn(Some("MAX".to_string())),
                        Err(_) => ColoredStatus::Error(Some("ERR".to_string())),
                    };
                    draw_status(ui, &dist_status, format!("DIST_{i}"));
                }
            }
        }
    }
}
