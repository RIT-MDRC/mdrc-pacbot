use crate::drawing::settings::{dropdown, num};
use crate::App;
use core_pb::names::RobotName;
use eframe::egui;
use eframe::egui::{Color32, Ui};
use egui_plot::{Legend, Line, LineStyle, Plot, PlotPoints, Points};

pub struct MotorStatusGraphFrames<const WHEELS: usize> {
    name: RobotName,
    first_x: Option<f64>,
    last_x: f64,
    pwm: [[Vec<[f64; 2]>; 2]; WHEELS],
    speeds: [Vec<[f64; 2]>; WHEELS],
    set_points: [Vec<[f64; 2]>; WHEELS],
}

impl<const WHEELS: usize> MotorStatusGraphFrames<WHEELS> {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            pwm: [0; WHEELS].map(|_| [vec![], vec![]]),
            speeds: [0; WHEELS].map(|_| vec![]),
            set_points: [0; WHEELS].map(|_| vec![]),
            first_x: None,
            last_x: 0.0,
        }
    }
}

pub fn draw_motors(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(format!(
            "Motor configuration for {}",
            app.ui_settings.selected_robot
        ));
        ui.separator();
        ui.checkbox(&mut app.ui_settings.record_motor_data, "Record data");
        if ui.button("Clear data").clicked() {
            app.motor_status_frames = MotorStatusGraphFrames::new(app.ui_settings.selected_robot);
        }
        if ui.button("Toggle all graphs").clicked() {
            if app
                .ui_settings
                .graph_lines
                .iter()
                .any(|x| x.iter().any(|x| !x))
            {
                app.ui_settings.graph_lines = [[true; 4]; 3];
            } else {
                app.ui_settings.graph_lines = [[false; 4]; 3];
            }
        }
        ui.separator();
        num(
            "motor_p".to_string(),
            ui,
            app.settings_fields.as_mut().unwrap(),
            &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                .config
                .pid[0],
            "P",
            true,
        );
        num(
            "motor_i".to_string(),
            ui,
            app.settings_fields.as_mut().unwrap(),
            &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                .config
                .pid[1],
            "I",
            true,
        );
        num(
            "motor_d".to_string(),
            ui,
            app.settings_fields.as_mut().unwrap(),
            &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                .config
                .pid[2],
            "D",
            true,
        );
    });
    ui.separator();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            for i in 0..3 {
                ui.horizontal(|ui| {
                    ui.label(format!("Motor {i}"));
                    if ui.button("Toggle graphs").clicked() {
                        if app.ui_settings.graph_lines[i].iter().any(|x| !x) {
                            app.ui_settings.graph_lines[i] = [true; 4];
                        } else {
                            app.ui_settings.graph_lines[i] = [false; 4];
                        }
                    }
                    ui.separator();
                    let current_override = &mut app.settings.robots
                        [app.ui_settings.selected_robot as usize]
                        .config
                        .motors_override[i];
                    let mut override_is_some = current_override.is_some();
                    ui.checkbox(&mut override_is_some, "Override");
                    if override_is_some && current_override.is_none() {
                        *current_override = Some(0.0);
                    } else if !override_is_some {
                        *current_override = None;
                    }
                    let mut def = app.server_status.robots[app.ui_settings.selected_robot as usize]
                        .last_motor_status
                        .1
                        .speed_set_points[i];
                    ui.add_enabled(
                        override_is_some,
                        egui::Slider::new(
                            current_override.as_mut().unwrap_or(&mut def),
                            -20.0..=20.0,
                        )
                        .text("Setpoint"),
                    );
                    ui.checkbox(&mut app.ui_settings.graph_lines[i][0], "Graph Setpoint");
                    ui.checkbox(&mut app.ui_settings.graph_lines[i][1], "Graph Speed");
                });
                egui::Grid::new(format!("motor_pins{i}")).show(ui, |ui| {
                    ui.label("Forwards pin: ");
                    dropdown(
                        ui,
                        format!("motor{i}_forwards"),
                        "",
                        &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                            .config
                            .motor_config[i][0],
                        &[0, 1, 2, 3, 4, 5],
                    );
                    let current_override = &mut app.settings.robots
                        [app.ui_settings.selected_robot as usize]
                        .config
                        .pwm_override[i][0];
                    let mut override_is_some = current_override.is_some();
                    ui.checkbox(&mut override_is_some, "Override");
                    if override_is_some && current_override.is_none() {
                        *current_override = Some(0);
                    } else if !override_is_some {
                        *current_override = None;
                    }
                    let mut def = app.server_status.robots[app.ui_settings.selected_robot as usize]
                        .last_motor_status
                        .1
                        .pwm[i][0];
                    ui.add_enabled(
                        override_is_some,
                        egui::Slider::new(
                            current_override.as_mut().unwrap_or(&mut def),
                            0..=app.ui_settings.selected_robot.robot().pwm_top,
                        )
                        .text("Set PWM"),
                    );
                    ui.checkbox(&mut app.ui_settings.graph_lines[i][2], "Graph PWM");
                    ui.end_row();

                    ui.label("Backwards pin: ");
                    dropdown(
                        ui,
                        format!("motor{i}_backwards"),
                        "",
                        &mut app.settings.robots[app.ui_settings.selected_robot as usize]
                            .config
                            .motor_config[i][1],
                        &[0, 1, 2, 3, 4, 5],
                    );
                    let current_override = &mut app.settings.robots
                        [app.ui_settings.selected_robot as usize]
                        .config
                        .pwm_override[i][1];
                    let mut override_is_some = current_override.is_some();
                    ui.checkbox(&mut override_is_some, "Override");
                    if override_is_some && current_override.is_none() {
                        *current_override = Some(0);
                    } else if !override_is_some {
                        *current_override = None;
                    }
                    let mut def = app.server_status.robots[app.ui_settings.selected_robot as usize]
                        .last_motor_status
                        .1
                        .pwm[i][1];
                    ui.add_enabled(
                        override_is_some,
                        egui::Slider::new(
                            current_override.as_mut().unwrap_or(&mut def),
                            0..=app.ui_settings.selected_robot.robot().pwm_top,
                        )
                        .text("Set PWM"),
                    );
                    ui.checkbox(&mut app.ui_settings.graph_lines[i][3], "Graph PWM");
                    ui.end_row();
                });
                ui.separator();
            }
        });
    });

    if app.ui_settings.record_motor_data {
        let (dur, status) =
            &app.server_status.robots[app.ui_settings.selected_robot as usize].last_motor_status;
        let x = dur.as_secs_f64();
        if app.motor_status_frames.first_x.is_none() {
            app.motor_status_frames.first_x = Some(x);
        }
        if app.ui_settings.selected_robot != app.motor_status_frames.name {
            app.motor_status_frames = MotorStatusGraphFrames::new(app.ui_settings.selected_robot);
        }
        if x < app.motor_status_frames.last_x {
            app.motor_status_frames = MotorStatusGraphFrames::new(app.ui_settings.selected_robot);
        }
        app.motor_status_frames.last_x = x;
        for i in 0..3 {
            app.motor_status_frames.pwm[i][0].push([
                x,
                100.0 * status.pwm[i][0] as f64
                    / app.ui_settings.selected_robot.robot().pwm_top as f64,
            ]);
            app.motor_status_frames.pwm[i][1].push([
                x,
                -100.0 * status.pwm[i][1] as f64
                    / app.ui_settings.selected_robot.robot().pwm_top as f64,
            ]);
            app.motor_status_frames.speeds[i].push([x, status.measured_speeds[i] as f64]);
            app.motor_status_frames.set_points[i].push([x, status.speed_set_points[i] as f64]);
        }
    }

    Plot::new("motor_plot")
        .x_axis_label("uptime (s)")
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            for m in 0..3 {
                let color = match m {
                    0 => (Color32::RED, Color32::DARK_RED),
                    1 => (Color32::GREEN, Color32::DARK_GREEN),
                    _ => (Color32::BLUE, Color32::DARK_BLUE),
                };
                let last_x = app.motor_status_frames.last_x;
                let first_x = app.motor_status_frames.first_x.unwrap_or(0.0);
                let mut extra_points = vec![[last_x, 0.0], [last_x, 3.0]];
                if last_x - first_x < 10.0 {
                    extra_points.push([first_x + 10.0, 0.0]);
                }
                plot_ui.points(Points::new(extra_points).color(app.background_color));
                if app.ui_settings.graph_lines[m][0] {
                    plot_ui.line(
                        Line::new(PlotPoints::new(
                            app.motor_status_frames.set_points[m].clone(),
                        ))
                        .name(format!("{m} Setpoint"))
                        .style(LineStyle::Dashed { length: 6.0 })
                        .color(color.0),
                    );
                }
                if app.ui_settings.graph_lines[m][1] {
                    plot_ui.line(
                        Line::new(PlotPoints::new(app.motor_status_frames.speeds[m].clone()))
                            .name(format!("{m} Speed"))
                            .color(color.0),
                    );
                }
                if app.ui_settings.graph_lines[m][2] {
                    plot_ui.line(
                        Line::new(PlotPoints::new(app.motor_status_frames.pwm[m][0].clone()))
                            .name(format!("{m}a PWM"))
                            .color(color.1),
                    );
                }
                if app.ui_settings.graph_lines[m][3] {
                    plot_ui.line(
                        Line::new(PlotPoints::new(app.motor_status_frames.pwm[m][1].clone()))
                            .name(format!("{m}b PWM"))
                            .color(color.1),
                    );
                }
            }
        });
}
