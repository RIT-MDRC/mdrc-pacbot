//! Records and replays GUI data

use crate::gui::{utils, AppMode, TabViewer};
use crate::replay::Replay;
use anyhow::Error;
use eframe::egui::Button;
use eframe::egui::Key;
use eframe::egui::Ui;
use native_dialog::FileDialog;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

impl<'a> TabViewer<'a> {
    /// Draw the UI involved in recording/playback
    ///
    /// ui should be just the bottom panel
    pub fn draw_replay_ui(&mut self, ctx: &eframe::egui::Context, ui: &mut Ui) {
        let game_paused = self.pacman_state.0.is_paused();

        let k_space = ctx.input(|i| i.key_pressed(Key::Space));
        let k_left = ctx.input(|i| i.key_pressed(Key::ArrowLeft));
        let k_right = ctx.input(|i| i.key_pressed(Key::ArrowRight));
        let k_shift = ctx.input(|i| i.modifiers.shift);

        utils::centered_group(ui, |ui| {
            let icon_button_size = eframe::egui::vec2(22.0, 22.0);
            let icon_button = |character| Button::new(character).min_size(icon_button_size);

            let playback_mode = matches!(self.settings.mode, AppMode::Playback);
            let advanced_controls = playback_mode;

            if ui
                .add_enabled(advanced_controls, icon_button("⏮"))
                .clicked()
                || (advanced_controls && k_left && k_shift)
            {
                self.replay_manager.replay.go_to_beginning();
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏪"))
                .clicked()
                || (advanced_controls && k_left && !k_shift)
            {
                self.replay_manager
                    .replay
                    .step_backwards_until_pacman_state();
            }
            if playback_mode {
                if self.replay_manager.playback_paused {
                    if ui.add_enabled(true, icon_button("▶")).clicked() || k_space {
                        self.replay_manager.playback_paused = false;
                    };
                } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                    self.replay_manager.playback_paused = true;
                }
            } else if game_paused {
                if ui.add_enabled(true, icon_button("▶")).clicked() || k_space {
                    self.pacman_state.0.unpause();
                }
            } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                self.pacman_state.0.pause();
            }
            if playback_mode {
                if ui
                    .add_enabled(advanced_controls, icon_button("☉"))
                    .clicked()
                {
                    self.replay_manager.replay = Replay::starting_at(&self.replay_manager.replay);
                    self.settings.mode = AppMode::Recording;
                }
            } else if ui.add_enabled(game_paused, icon_button("⏹")).clicked() {
                self.settings.mode = AppMode::Playback;
            }
            if ui
                .add_enabled(
                    advanced_controls || (!playback_mode && game_paused),
                    icon_button("⏩"),
                )
                .clicked()
                || ((advanced_controls || (!playback_mode && game_paused)) && k_right && !k_shift)
            {
                if playback_mode {
                    self.replay_manager
                        .replay
                        .step_forwards_until_pacman_state();
                } else {
                    // game is live but paused
                    {
                        self.pacman_state.0.unpause();
                        self.pacman_state.0.force_step();
                        self.pacman_state.0.pause();
                    }
                    self.replay_manager
                        .replay
                        .record_pacman_state(&self.pacman_state.0)
                        .expect("Failed to record pacman state!");
                }
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏭"))
                .clicked()
                || (advanced_controls && k_right && k_shift)
            {
                self.replay_manager.replay.go_to_end();
            }

            ui.add_enabled(
                playback_mode,
                eframe::egui::Slider::new(&mut self.replay_manager.playback_speed, -5.0..=5.0)
                    .text("Playback Speed"),
            );
        });
    }

    /// Save the current replay to file
    pub fn save_replay(&self) -> Result<(), Error> {
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .set_filename("replay.pb")
            .show_save_single_file()?;

        if let Some(path) = path {
            let bytes = self.replay_manager.replay.to_bytes()?;
            let mut file = fs::OpenOptions::new().write(true).create(true).open(path)?;
            file.write_all(&bytes)?;
        }

        Ok(())
    }

    /// Load a replay from file
    pub fn load_replay(&mut self) -> Result<(), Error> {
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .show_open_single_file()?;

        if let Some(path) = path {
            let mut file = File::open(&path)?;
            let metadata = fs::metadata(&path).expect("unable to read metadata");
            let mut buffer = vec![0; metadata.len() as usize];
            file.read_exact(&mut buffer)?;

            let replay = Replay::from_bytes(&buffer)?;

            self.settings.mode = AppMode::Playback;
            self.replay_manager.replay = replay;
            self.replay_manager.playback_paused = true;
        }

        Ok(())
    }
}
