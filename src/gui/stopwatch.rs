use crate::gui::{PacbotWidget, PacbotWidgetStatus, Tab};
use crate::util::stopwatch::Stopwatch;
use eframe::egui::RichText;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

const ACCEPTABLE_RECORDING_DELAY: Duration = Duration::from_millis(20);

#[derive(Clone, Debug)]
struct GuiStopwatch {
    display_name: String,
    stopwatch: Arc<RwLock<Stopwatch>>,
    last_recorded_average_millis: f32,
    last_recorded_at: Instant,
    ok_time_millis: f32,
    bad_time_millis: f32,
}

impl GuiStopwatch {
    pub fn new(
        display_name: &'static str,
        samples: usize,
        ok_time_millis: f32,
        bad_time_millis: f32,
    ) -> Self {
        Self {
            display_name: display_name.to_string(),
            stopwatch: Arc::new(RwLock::new(Stopwatch::new(samples))),
            last_recorded_average_millis: 0.0,
            last_recorded_at: Instant::now(),
            ok_time_millis,
            bad_time_millis,
        }
    }
}

pub(super) struct StopwatchWidget {
    stopwatches: Vec<GuiStopwatch>,
    pf_time: f32,
    status: PacbotWidgetStatus,
    messages: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl StopwatchWidget {
    pub fn new() -> (Self, [Arc<RwLock<Stopwatch>>; 3]) {
        let stopwatches = vec![
            GuiStopwatch::new("GUI", 30, 20.0, 30.0),
            GuiStopwatch::new("Physics", 10, 4.0, 6.0),
            GuiStopwatch::new("PF", 10, 4.0, 6.0),
        ];
        (
            Self {
                stopwatches: stopwatches.clone(),
                pf_time: 999.99,
                status: PacbotWidgetStatus::Ok,
                messages: vec![],
                warnings: vec![],
                errors: vec![],
            },
            [
                stopwatches[0].stopwatch.clone(),
                stopwatches[1].stopwatch.clone(),
                stopwatches[2].stopwatch.clone(),
            ],
        )
    }
}

impl PacbotWidget for StopwatchWidget {
    fn update(&mut self) {
        self.messages = vec![];
        self.warnings = vec![];
        self.errors = vec![];
        let mut num_too_slow = 0;
        let mut num_slow = 0;
        let mut num_no_data = 0;
        for stopwatch in &mut self.stopwatches {
            if let Ok(s) = stopwatch.stopwatch.read() {
                stopwatch.last_recorded_average_millis = s.average_process_time() * 1000.0;
                stopwatch.last_recorded_at = Instant::now();

                let t = stopwatch.last_recorded_average_millis;
                let msg = format!("{:.2} - {}", t, stopwatch.display_name);
                if stopwatch.ok_time_millis < t && t < stopwatch.bad_time_millis {
                    self.warnings.push(msg);
                    num_slow += 1;
                } else if t > stopwatch.bad_time_millis {
                    self.errors.push(msg);
                    num_too_slow += 1;
                } else {
                    self.messages.push(msg);
                }

                if stopwatch.display_name == "PF" {
                    self.pf_time = t;
                }
            } else {
                if stopwatch.last_recorded_at.elapsed() > ACCEPTABLE_RECORDING_DELAY {
                    self.errors.push(format!(
                        "{}: No data for {}",
                        stopwatch.display_name,
                        stopwatch.last_recorded_at.elapsed().as_millis()
                    ));
                    num_no_data += 1;
                }
            }
        }
        self.status = if num_no_data > 0 {
            PacbotWidgetStatus::Error(format!("no data for {} watches", num_no_data))
        } else if num_too_slow > 0 {
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

    fn messages(&self) -> &[String] {
        &self.messages
    }

    fn warnings(&self) -> &[String] {
        &self.warnings
    }

    fn errors(&self) -> &[String] {
        &self.errors
    }
}
