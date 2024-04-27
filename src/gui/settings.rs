use crate::gui::{PacbotWidget, Tab, TabViewer};
use bevy_egui::egui::RichText;
use eframe::egui;
use eframe::egui::Ui;
use egui_phosphor::regular;

use crate::{CvPositionSource, HighLevelStrategy};
use num_traits::Num;
use rapier2d::na::Vector2;

fn int_edit(ui: &mut Ui, label: &str, initial: &mut usize) {
    ui.label(label);
    let mut mutint = (*initial).to_string();
    ui.text_edit_singleline(&mut mutint);
    if let Ok(i) = mutint.as_str().parse::<usize>() {
        *initial = i;
    }
}

fn u8_edit(ui: &mut Ui, label: &str, initial: &mut u8) {
    ui.label(label);
    let mut mutint = (*initial).to_string();
    ui.text_edit_singleline(&mut mutint);
    if let Ok(i) = mutint.as_str().parse::<u8>() {
        *initial = i;
    }
}

fn f32_edit(ui: &mut Ui, label: &str, initial: &mut f32) {
    ui.label(label);
    let mut mutint = (*initial).to_string();
    ui.text_edit_singleline(&mut mutint);
    if let Ok(i) = f32::from_str_radix(mutint.as_str(), 10) {
        *initial = i;
    }
}

impl<'a> TabViewer<'a> {
    pub fn draw_settings(&mut self, ui: &mut Ui) {
        ui.label("Settings");
        ui.separator();
        let old_strategy = self.settings.high_level_strategy;
        egui::ComboBox::from_label("Strategy ")
            .selected_text(match self.settings.high_level_strategy {
                HighLevelStrategy::Manual => "Manual",
                HighLevelStrategy::ReinforcementLearning => "AI",
                HighLevelStrategy::TestUniform => "Test (Uniform)",
                HighLevelStrategy::TestNonExplored => "Test (Non Explored)",
                HighLevelStrategy::TestForward => "Test (Forwards)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.settings.high_level_strategy,
                    HighLevelStrategy::Manual,
                    "Manual",
                );
                ui.selectable_value(
                    &mut self.settings.high_level_strategy,
                    HighLevelStrategy::ReinforcementLearning,
                    "AI",
                );
                ui.selectable_value(
                    &mut self.settings.high_level_strategy,
                    HighLevelStrategy::TestUniform,
                    "Test (Uniform)",
                );
                ui.selectable_value(
                    &mut self.settings.high_level_strategy,
                    HighLevelStrategy::TestNonExplored,
                    "Test (Non Explored)",
                );
                ui.selectable_value(
                    &mut self.settings.high_level_strategy,
                    HighLevelStrategy::TestForward,
                    "Test (Forwards)",
                );
            });
        if self.settings.high_level_strategy != old_strategy {
            self.settings.test_path_position = None;
            self.target_path.0.clear();
            self.target_velocity.0 = Vector2::new(0.0, 0.0);
            self.target_velocity.1 = 0.0;
        }
        ui.checkbox(&mut self.settings.enable_pf, "PF enabled");
        ui.checkbox(
            &mut self.settings.collision_avoidance,
            "Collision avoidance",
        );
        int_edit(ui, "Bot Update Period",&mut self.settings.bot_update_period);
        ui.separator();

        let mut pico_enabled = self.settings.pico_address.is_some();
        ui.checkbox(&mut pico_enabled, "Pico enabled");
        if !pico_enabled {
            self.settings.pico_address = None;
        } else {
            let mut pico_addr = self
                .settings
                .pico_address
                .clone()
                .unwrap_or("10.181.92.51:20002".to_string());
            ui.text_edit_singleline(&mut pico_addr);
            self.settings.pico_address = Some(pico_addr);
        }
        self.settings.sensors_from_robot = self.settings.pico_address.is_some();
        ui.checkbox(
            &mut self.settings.motors_ignore_phys_angle,
            "Motor commands ignore physics angle",
        );

        ui.separator();

        f32_edit(ui, "[P]ID", &mut self.settings.pid[0]);
        f32_edit(ui, "P[I]D", &mut self.settings.pid[1]);
        f32_edit(ui, "PI[D]", &mut self.settings.pid[2]);

        ui.separator();

        f32_edit(ui, "base speed", &mut self.settings.speed_base);
        f32_edit(ui, "speed multiplier", &mut self.settings.speed_multiplier);
        f32_edit(ui, "speed cap", &mut self.settings.speed_cap);
        f32_edit(ui, "max accel", &mut self.settings.max_accel);

        f32_edit(ui, "manual speed", &mut self.settings.manual_speed);
        f32_edit(ui, "rotate speed", &mut self.settings.manual_rotate_speed);

        u8_edit(
            ui,
            "collision distance threshold",
            &mut self.settings.collision_distance_threshold,
        );
        u8_edit(
            ui,
            "collision distance stop",
            &mut self.settings.collision_distance_stop,
        );
        u8_edit(
            ui,
            "distance sensor interval",
            &mut self.settings.sensor_range_interval,
        );

        ui.separator();

        egui::ComboBox::from_label("CV source")
            .selected_text(format!("{:?}", self.settings.cv_position))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::DelayedGameState(0.0.into()),
                    "GameState",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::ParticleFilter,
                    "ParticleFilter",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::DelayedGameState(1.0.into()),
                    "DelayedGS(1)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(1, 1),
                    "(1, 1)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(1, 5),
                    "(1, 5)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(10, 5),
                    "O: (10, 5)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(20, 5),
                    "T: (20, 5)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(27, 11),
                    "+: (27, 11)",
                );
                ui.selectable_value(
                    &mut self.settings.cv_position,
                    CvPositionSource::Constant(27, 24),
                    "8: (27, 24)",
                );
            });

        ui.separator();

        let mut go_server_enabled = self.settings.go_server_address.is_some();
        ui.checkbox(&mut go_server_enabled, "Go server enabled");
        if !go_server_enabled {
            self.settings.go_server_address = None;
        } else {
            let mut go_addr = self
                .settings
                .go_server_address
                .clone()
                .unwrap_or("".to_string());
            ui.text_edit_singleline(&mut go_addr);
            self.settings.go_server_address = Some(go_addr);
            self.reconnect = ui.button("Connect").clicked();
            ui.label(if self.connected {
                "Connected"
            } else {
                "Not Connected"
            });
        }

        ui.separator();
        ui.label("Robot settings coming soon!");
        ui.separator();

        f32_edit(
            ui,
            "Noise proportional to translation",
            &mut self.settings.pf_simulated_translation_noise,
        );
        f32_edit(
            ui,
            "Noise proportional to rotation",
            &mut self.settings.pf_simulated_rotation_noise,
        );
        f32_edit(
            ui,
            "Noise for movement in general",
            &mut self.settings.pf_generic_noise,
        );
        f32_edit(
            ui,
            "The average number of times the robot is kidnapped per second, in our theoretical motion model",
            &mut self.settings.pf_avg_kidnaps_per_sec,
        );
        f32_edit(
            ui,
            "The standard deviation of the CV position error, in our theoretical sensor model",
            &mut self.settings.pf_cv_error_std,
        );
        f32_edit(
            ui,
            "The standard deviation of the distance sensor errors, in our theoretical sensor model",
            &mut self.settings.pf_sensor_error_std,
        );

        ui.separator();

        ui.checkbox(
            &mut self.settings.replay_save_location,
            "Save physical location to replay",
        );
        ui.checkbox(
            &mut self.settings.replay_save_targets,
            "Save target path and velocity to replay",
        );
        ui.checkbox(
            &mut self.settings.replay_save_sensors,
            "Save sensors to replay",
        );

        ui.separator();

        int_edit(ui, "Total points", &mut self.settings.pf_total_points);
        int_edit(ui, "Displayed points", &mut self.settings.pf_gui_points);
        f32_edit(
            ui,
            "Chance to spawn near another",
            &mut self.settings.pf_chance_near_other,
        );

        f32_edit(
            ui,
            "Translation change limit",
            &mut self.settings.pf_translation_limit,
        );
        f32_edit(
            ui,
            "Rotation change limit",
            &mut self.settings.pf_rotation_limit,
        );
    }
}

#[derive(Clone, Default)]
pub struct PacbotSettingsWidget;

impl PacbotWidget for PacbotSettingsWidget {
    fn display_name(&self) -> &'static str {
        "Settings"
    }

    fn button_text(&self) -> RichText {
        RichText::new(regular::GEAR.to_string())
    }

    fn tab(&self) -> Tab {
        Tab::Settings
    }
}
