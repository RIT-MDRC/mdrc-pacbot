use crate::gui::{PacbotWidget, PacbotWidgetStatus, Tab, TabViewer};
use eframe::egui::RichText;

pub(super) struct StopwatchWidget {
    pf_time: f32,
    status: PacbotWidgetStatus,
    messages: Vec<(String, PacbotWidgetStatus)>,
}

impl StopwatchWidget {
    pub fn new() -> Self {
        Self {
            pf_time: 999.99,
            status: PacbotWidgetStatus::Ok,
            messages: vec![],
        }
    }
}

impl PacbotWidget for StopwatchWidget {
    fn update(&mut self, tab_viewer: &TabViewer) {
        self.messages = vec![];
        let mut num_too_slow = 0;
        let mut num_slow = 0;
        for stopwatch in [
            &tab_viewer.physics_stopwatch.0,
            &tab_viewer.pf_stopwatch.0,
            &tab_viewer.gui_stopwatch.0,
            &tab_viewer.schedule_stopwatch.0,
        ] {
            let t = stopwatch.average_process_time() * 1000.0;
            let msg = format!("{:.2} - {}", t, stopwatch.display_name());
            if stopwatch.ok_time_millis() < t && t < stopwatch.bad_time_millis() {
                self.messages
                    .push((msg, PacbotWidgetStatus::Warn("".to_string())));
                num_slow += 1;
            } else if t > stopwatch.bad_time_millis() {
                self.messages
                    .push((msg, PacbotWidgetStatus::Error("".to_string())));
                num_too_slow += 1;
            } else {
                self.messages.push((msg, PacbotWidgetStatus::Ok));
            }

            if stopwatch.display_name() == "PF" {
                self.pf_time = t;
            }
        }
        self.status = if num_too_slow > 0 {
            PacbotWidgetStatus::Error(format!("{} watches were too slow", num_too_slow))
        } else if num_slow > 0 {
            PacbotWidgetStatus::Warn(format!("{} watches were slow", num_slow))
        } else {
            PacbotWidgetStatus::Ok
        };
    }

    fn display_name(&self) -> &'static str {
        "Timing"
    }

    fn button_text(&self) -> RichText {
        RichText::new(format!(
            "{} {:.2}",
            egui_phosphor::regular::TIMER,
            self.pf_time
        ))
    }

    fn tab(&self) -> Tab {
        Tab::Stopwatch
    }

    fn overall_status(&self) -> &PacbotWidgetStatus {
        &self.status
    }

    fn messages(&self) -> &[(String, PacbotWidgetStatus)] {
        &self.messages
    }
}
