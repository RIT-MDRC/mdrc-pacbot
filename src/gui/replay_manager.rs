//! Records and replays GUI data

use crate::gui::{utils, AppMode, TabViewer};
use crate::replay::Replay;
use crate::replay_manager::ReplayManager;
use crate::UserSettings;
use anyhow::Error;
use eframe::egui::Button;
use eframe::egui::Key;
use eframe::egui::Ui;
use native_dialog::FileDialog;
use pacbot_rs::game_engine::GameEngine;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

impl<'a> TabViewer<'a> {
    /// Draw the UI involved in recording/playback
    ///
    /// ui should be just the bottom panel
    pub fn draw_replay_ui(
        &mut self,
        ctx: &eframe::egui::Context,
        ui: &mut Ui,
        pacman_state: &mut GameEngine,
        replay_manager: &mut ReplayManager,
        settings: &mut UserSettings,
    ) {
        let game_paused = pacman_state.is_paused();

        let k_space = ctx.input(|i| i.key_pressed(Key::Space));
        let k_left = ctx.input(|i| i.key_pressed(Key::ArrowLeft));
        let k_right = ctx.input(|i| i.key_pressed(Key::ArrowRight));
        let k_shift = ctx.input(|i| i.modifiers.shift);

        utils::centered_group(ui, |ui| {
            let icon_button_size = eframe::egui::vec2(22.0, 22.0);
            let icon_button = |character| Button::new(character).min_size(icon_button_size);

            let playback_mode = matches!(settings.mode, AppMode::Playback);
            let advanced_controls = playback_mode;

            if ui
                .add_enabled(advanced_controls, icon_button("⏮"))
                .clicked()
                || (k_left && k_shift)
            {
                replay_manager.replay.go_to_beginning();
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏪"))
                .clicked()
                || (k_left && !k_shift)
            {
                replay_manager.replay.step_backwards_until_pacman_state();
            }
            if playback_mode {
                if replay_manager.playback_paused {
                    if ui.add_enabled(true, icon_button("▶")).clicked() || k_space {
                        replay_manager.playback_paused = false;
                    };
                } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                    replay_manager.playback_paused = true;
                }
            } else if game_paused {
                if ui.add_enabled(true, icon_button("▶")).clicked() || k_space {
                    pacman_state.unpause();
                }
            } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                pacman_state.pause();
            }
            if playback_mode {
                if ui
                    .add_enabled(advanced_controls, icon_button("☉"))
                    .clicked()
                {
                    replay_manager.replay = Replay::starting_at(&replay_manager.replay);
                    settings.mode = AppMode::Recording;
                }
            } else if ui.add_enabled(game_paused, icon_button("⏹")).clicked() {
                settings.mode = AppMode::Playback;
            }
            if ui
                .add_enabled(
                    advanced_controls || (!playback_mode && game_paused),
                    icon_button("⏩"),
                )
                .clicked()
                || (k_right && !k_shift)
            {
                if playback_mode {
                    replay_manager.replay.step_forwards_until_pacman_state();
                } else {
                    // game is live but paused
                    {
                        pacman_state.unpause();
                        pacman_state.force_step();
                        pacman_state.pause();
                    }
                    replay_manager
                        .replay
                        .record_pacman_state(&pacman_state)
                        .expect("Failed to record pacman state!");
                }
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏭"))
                .clicked()
                || (k_right && k_shift)
            {
                replay_manager.replay.go_to_end();
            }

            ui.add_enabled(
                playback_mode,
                eframe::egui::Slider::new(&mut replay_manager.playback_speed, -5.0..=5.0)
                    .text("Playback Speed"),
            );
        });
    }

    /// Save the current replay to file
    pub fn save_replay(&self, replay_manager: &ReplayManager) -> Result<(), Error> {
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .set_filename("replay.pb")
            .show_save_single_file()?;

        if let Some(path) = path {
            let bytes = replay_manager.replay.to_bytes()?;
            let mut file = fs::OpenOptions::new().write(true).create(true).open(path)?;
            file.write_all(&bytes)?;
        }

        Ok(())
    }

    /// Load a replay from file
    pub fn load_replay(
        &mut self,
        replay_manager: &mut ReplayManager,
        settings: &mut UserSettings,
    ) -> Result<(), Error> {
        let path = FileDialog::new()
            .add_filter("Pacbot Replay", &["pb"])
            .show_open_single_file()?;

        if let Some(path) = path {
            let mut file = File::open(&path)?;
            let metadata = fs::metadata(&path).expect("unable to read metadata");
            let mut buffer = vec![0; metadata.len() as usize];
            file.read_exact(&mut buffer)?;

            let replay = Replay::from_bytes(&buffer)?;

            settings.mode = AppMode::Playback;
            replay_manager.replay = replay;
            replay_manager.playback_paused = true;
        }

        Ok(())
    }
}
