use crate::drawing::settings::dropdown;
use crate::App;
use eframe::egui;
use eframe::egui::{Color32, Pos2, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints};

pub fn draw_motors(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(format!(
            "Motor configuration for {}",
            app.ui_settings.selected_robot
        ));
        ui.separator();
        // ui.checkbox(&mut motor_config[0].2, "Record data");
        // ui.button("Clear data").clicked();
    });
    ui.separator();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            for i in 0..3 {
                ui.horizontal(|ui| {
                    ui.label(format!("Motor {i}"));
                    // ui.separator();
                    // ui.add(egui::Slider::new(&mut x, -10.0..=10.0).text("Setpoint"));
                    // ui.button("Reset").clicked();
                });
                egui::Grid::new(format!("motor_pins{i}")).show(ui, |ui| {
                    ui.label("Forwards pin: ");
                    dropdown(
                        ui,
                        format!("motor{i}_forwards"),
                        "",
                        &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                            .motor_config[i][0],
                        &[0, 1, 2, 3, 4, 5],
                    );
                    let current_override = &mut app.settings.robots
                        [app.ui_settings.selected_robot as usize]
                        .pwm_override[i][0];
                    let mut override_is_some = current_override.is_some();
                    ui.checkbox(&mut override_is_some, "Override");
                    if override_is_some && current_override.is_none() {
                        *current_override = Some(0);
                    } else if !override_is_some {
                        *current_override = None;
                    }
                    ui.add_enabled(
                        override_is_some,
                        egui::Slider::new(
                            current_override.as_mut().unwrap_or(&mut 0),
                            0..=app.ui_settings.selected_robot.robot().pwm_top,
                        )
                        .text("Set PWM"),
                    );
                    ui.end_row();

                    ui.label("Backwards pin: ");
                    dropdown(
                        ui,
                        format!("motor{i}_backwards"),
                        "",
                        &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                            .motor_config[i][1],
                        &[0, 1, 2, 3, 4, 5],
                    );
                    let current_override = &mut app.settings.robots
                        [app.ui_settings.selected_robot as usize]
                        .pwm_override[i][1];
                    let mut override_is_some = current_override.is_some();
                    ui.checkbox(&mut override_is_some, "Override");
                    if override_is_some && current_override.is_none() {
                        *current_override = Some(0);
                    } else if !override_is_some {
                        *current_override = None;
                    }
                    ui.add_enabled(
                        override_is_some,
                        egui::Slider::new(
                            current_override.as_mut().unwrap_or(&mut 0),
                            0..=app.ui_settings.selected_robot.robot().pwm_top,
                        )
                        .text("Set PWM"),
                    );
                    ui.end_row();
                });
                ui.separator();
            }
        });
    });
    let pwm1_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("1 Speed")
        .color(Color32::RED);
    let pwm2_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("2 Speed")
        .color(Color32::BLUE);
    let pwm3_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("3 Speed")
        .color(Color32::GREEN);
    let setpnt1_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("1 Setpoint")
        .color(Color32::RED);
    let setpnt2_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("2 Setpoint")
        .color(Color32::BLUE);
    let setpnt3_line = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("3 Setpoint")
        .color(Color32::GREEN);
    let pid_line1a = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("1a PID Output")
        .color(Color32::DARK_RED);
    let pid_line2a = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("2a PID Output")
        .color(Color32::DARK_BLUE);
    let pid_line3a = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("3a PID Output")
        .color(Color32::DARK_GREEN);
    let pid_line1b = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("1b PID Output")
        .color(Color32::DARK_RED);
    let pid_line2b = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("2b PID Output")
        .color(Color32::DARK_BLUE);
    let pid_line3b = Line::new(PlotPoints::new(vec![[0.0, 0.0]]))
        .name("3b PID Output")
        .color(Color32::DARK_GREEN);
    Plot::new("motor_plot")
        .x_axis_label("t (s)")
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(pwm1_line);
            plot_ui.line(pwm2_line);
            plot_ui.line(pwm3_line);
            plot_ui.line(setpnt1_line);
            plot_ui.line(setpnt2_line);
            plot_ui.line(setpnt3_line);
            plot_ui.line(pid_line1a);
            plot_ui.line(pid_line2a);
            plot_ui.line(pid_line3a);
            plot_ui.line(pid_line1b);
            plot_ui.line(pid_line2b);
            plot_ui.line(pid_line3b);
        });
}
