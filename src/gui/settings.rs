use crate::gui::{PacbotWidget, Tab, TabViewer};
use bevy_egui::egui::RichText;
use eframe::egui::Ui;
use egui_phosphor::regular;

use num_traits::Num;

fn int_edit(ui: &mut Ui, label: &str, initial: &mut usize) {
    ui.label(label);
    let mut mutint = (*initial).to_string();
    ui.text_edit_singleline(&mut mutint);
    if let Ok(i) = usize::from_str_radix(mutint.as_str(), 10) {
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
        ui.checkbox(&mut self.settings.enable_ai, "AI enabled");
        ui.checkbox(&mut self.settings.enable_pf, "PF enabled");
        ui.separator();

        let mut pico_enabled = self.settings.pico_address.is_some();
        ui.checkbox(&mut pico_enabled, "Pico enabled");
        if !pico_enabled {
            self.settings.pico_address = None;
        } else {
            let mut pico_addr = self.settings.pico_address.clone().unwrap_or("".to_string());
            ui.text_edit_singleline(&mut pico_addr);
            self.settings.pico_address = Some(pico_addr);
        }

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
        f32_edit(ui, "Error threshold", &mut self.settings.pf_error_threshold);
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
        RichText::new(format!("{}", regular::GEAR,))
    }

    fn tab(&self) -> Tab {
        Tab::Settings
    }
}
