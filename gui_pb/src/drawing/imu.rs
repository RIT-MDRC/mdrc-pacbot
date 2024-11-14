use crate::App;
use eframe::egui::Ui;

fn status_label(status: u8) -> &'static str {
    match status & 0x03 {
        0 => "Unreliable",
        1 => "Low",
        2 => "Medium",
        3 => "High",
        _ => "Unknown",
    }
}

pub fn draw_imu_data(app: &mut App, ui: &mut Ui) {
    let name = app.ui_settings.selected_robot;
    ui.heading(format!("IMU data for {name}"));
    ui.separator();
    if let Some(data) = app.server_status.robots[name as usize].extra_imu_data {
        for (label, ([x, y, z], status)) in [
            ("Accelerometer", data.accel),
            ("Gyroscope", data.gyro),
            ("Magnetometer", data.mag),
        ] {
            ui.label(format!("{label} x: {x}"));
            ui.label(format!("{label} y: {y}"));
            ui.label(format!("{label} z: {z}"));
            ui.label(format!("{label} accuracy status: {}", status_label(status)));
            ui.separator();
        }
        ui.label(format!("Rotation x: {}", data.rotation_vector.0[0]));
        ui.label(format!("Rotation y: {}", data.rotation_vector.0[1]));
        ui.label(format!("Rotation z: {}", data.rotation_vector.0[2]));
        ui.label(format!("Rotation w: {}", data.rotation_vector.0[3]));
        ui.label(format!(
            "Rotation accuracy (radians): {}",
            data.rotation_vector.1
        ));
        ui.label(format!(
            "Rotation status: {}",
            status_label(data.rotation_vector.2)
        ));
    }
}
