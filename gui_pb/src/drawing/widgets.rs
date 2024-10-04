use crate::App;
use core_pb::util::ColoredStatus;
use eframe::egui;
use eframe::egui::{RichText, Ui};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum PacbotWidget {
    GridWidget,
    UtilizationWidget,
}

pub fn draw_widgets(app: &mut App, ui: &mut Ui) {
    for widget_name in [PacbotWidget::GridWidget, PacbotWidget::UtilizationWidget] {
        let button = ui.add(
            egui::Button::new(widget_name.button_text(app))
                .fill(widget_name.overall_status(app).to_color32()),
        );
        if widget_name == PacbotWidget::UtilizationWidget {
            button.on_hover_ui(|ui| widget_name.hover_ui(app, ui));
        }
    }
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
        }
    }

    pub fn hover_ui(&self, app: &mut App, ui: &mut Ui) {
        #[allow(clippy::single_match)]
        match self {
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
                for (status, label) in [
                    (&app.server_status.utilization, "Server"),
                    (&app.server_status.inference_time, "Inference"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(status.icon()).color(status.to_color32_solid()));
                        ui.label(format!(
                            "{label}: {}",
                            status.message().unwrap_or("?".to_string())
                        ));
                    });
                }
            }
            _ => {}
        }
    }
}
