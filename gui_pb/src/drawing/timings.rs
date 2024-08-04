use crate::App;
use core_pb::util::stopwatch::Stopwatch;
use core_pb::util::CrossPlatformInstant;
use eframe::egui;
use eframe::egui::Ui;

fn draw_stopwatch<const SEGMENTS: usize, const WINDOW: usize, I: CrossPlatformInstant + Default>(
    stopwatch: &Stopwatch<SEGMENTS, WINDOW, I>,
    ui: &mut Ui,
    id: String,
) {
    ui.label(format!(
        "{} Total: {:.2?}",
        stopwatch.name(),
        stopwatch.process_average(),
    ));
    ui.separator();
    egui::Grid::new(id)
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let segment_times = stopwatch.segment_averages();
            for (name, time) in segment_times {
                ui.label(name.unwrap_or("?"));
                ui.label(format!("{:.2?}", time));
                ui.end_row();
            }
        });
}

pub fn draw_timings(app: &mut App, ui: &mut Ui) {
    draw_stopwatch(&app.gui_stopwatch, ui, "gui_stopwatch".to_string());
    ui.separator();
    ui.label(format!(
        "Server: {}",
        app.server_status
            .utilization
            .message()
            .unwrap_or("?".to_string())
    ));
}
