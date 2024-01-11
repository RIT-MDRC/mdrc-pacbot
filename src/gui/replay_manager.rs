//! Records and replays GUI data

use crate::grid::standard_grids::StandardGrid;
use crate::gui::{utils, AppMode, TabViewer};
use crate::replay::Replay;
use anyhow::Error;
use eframe::egui::Button;
use eframe::egui::Key;
use eframe::egui::Ui;
use native_dialog::FileDialog;
use pacbot_rs::game_engine::GameEngine;
use rapier2d::math::Rotation;
use rapier2d::na::Isometry2;
use rapier2d::prelude::Translation;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, SystemTime};

/// The public interface for recording and replaying GUI data
pub struct ReplayManager {
    /// The current replay, which may be recording or playing back
    replay: Replay,
    /// When current_frame was played; used to determine when to advance the replay
    playback_time: SystemTime,
    /// Whether playback is paused
    playback_paused: bool,
    /// Speed of playback - 0 is stopped, 1 is normal forwards
    playback_speed: f32,
}

impl TabViewer {
    /// Create a new ReplayManager; assumes that it is starting in recording mode
    ///
    /// Note: pacman_state is copied once to initialize the replay
    pub fn new_replay_manager(
        filename: String,
        standard_grid: StandardGrid,
        pacman_state: GameEngine,
        pacbot_location: Isometry2<f32>,
    ) -> ReplayManager {
        let replay = Replay::new(
            filename.to_owned(),
            standard_grid,
            pacman_state,
            pacbot_location,
        );
        ReplayManager {
            replay,
            playback_time: SystemTime::now(),
            playback_paused: true,
            playback_speed: 1.0,
        }
    }

    /// Play back or save frames as necessary
    ///
    /// When not in Playback mode, update_replay_playback has no effect
    pub fn update_replay_manager(&mut self) -> Result<(), Error> {
        // did pacman state request saving?
        if self.pacman_state_notify_recv.try_recv().is_ok() && self.mode != AppMode::Playback {
            let state = self.pacman_render.read().unwrap().pacman_state.to_owned();
            // if we aren't recording the physics position, we should record the game position
            if !self.save_pacbot_location {
                self.replay_manager
                    .replay
                    .record_pacman_location(Isometry2::from_parts(
                        Translation::new(
                            state.get_state().pacman_loc.row as f32,
                            state.get_state().pacman_loc.col as f32,
                        ),
                        Rotation::new(match state.get_state().pacman_loc.dir {
                            pacbot_rs::location::RIGHT => std::f32::consts::FRAC_PI_2,
                            pacbot_rs::location::UP => std::f32::consts::PI,
                            pacbot_rs::location::LEFT => std::f32::consts::FRAC_PI_2 * 3.0,
                            pacbot_rs::location::DOWN => 0.0,
                            _ => 0.0,
                        }),
                    ))?;
            }
            self.replay_manager.replay.record_pacman_state(state)?;
        }

        if self.mode != AppMode::Playback && self.save_pacbot_location {
            // save physics position
            let position = self.phys_render.read().unwrap().pacbot_pos;
            self.replay_manager
                .replay
                .record_pacman_location(position)?;
        }

        if self.replay_manager.playback_paused {
            // When playback is paused, constantly set this to now so that it starts up correctly
            self.replay_manager.playback_time = SystemTime::now();
            return Ok(());
        }

        if self.replay_manager.replay.is_at_end() {
            self.replay_manager.playback_paused = true;
            // we have reached the end of the replay
            return Ok(());
        }

        let now = SystemTime::now();

        if self.replay_manager.playback_speed >= 0.0 {
            loop {
                let time_to_next = self.replay_manager.replay.time_to_next().as_secs_f32();
                let should_step_replay = time_to_next / self.replay_manager.playback_speed
                    < now
                        .duration_since(self.replay_manager.playback_time)?
                        .as_secs_f32();

                if !should_step_replay {
                    break;
                }

                self.replay_manager.replay.step_forwards();
                self.replay_manager.playback_time +=
                    Duration::from_secs_f32(time_to_next / self.replay_manager.playback_speed);
            }
        } else {
            loop {
                let time_to_previous = self.replay_manager.replay.time_to_previous().as_secs_f32();
                let should_step_replay = time_to_previous / -self.replay_manager.playback_speed
                    < now
                        .duration_since(self.replay_manager.playback_time)?
                        .as_secs_f32();

                if !should_step_replay {
                    break;
                }

                self.replay_manager.replay.step_back();
                self.replay_manager.playback_time +=
                    Duration::from_secs_f32(time_to_previous / -self.replay_manager.playback_speed);
            }
        }

        self.update_with_replay();

        Ok(())
    }

    /// Draw the UI involved in recording/playback
    ///
    /// ui should be just the bottom panel
    pub fn draw_replay_ui(&mut self, ctx: &eframe::egui::Context, ui: &mut Ui) {
        let game_paused = self.pacman_render.write().unwrap().pacman_state.is_paused();

        let k_space = ctx.input(|i| i.key_pressed(Key::Space));
        let k_left = ctx.input(|i| i.key_pressed(Key::ArrowLeft));
        let k_right = ctx.input(|i| i.key_pressed(Key::ArrowRight));
        let k_shift = ctx.input(|i| i.modifiers.shift);

        utils::centered_group(ui, |ui| {
            let icon_button_size = eframe::egui::vec2(22.0, 22.0);
            let icon_button = |character| Button::new(character).min_size(icon_button_size);

            let playback_mode = matches!(self.mode, AppMode::Playback);
            let advanced_controls = playback_mode;

            if ui
                .add_enabled(advanced_controls, icon_button("⏮"))
                .clicked()
                || (k_left && k_shift)
            {
                self.replay_manager.replay.go_to_beginning();
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏪"))
                .clicked()
                || (k_left && !k_shift)
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
                    self.pacman_render.write().unwrap().pacman_state.unpause();
                }
            } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                self.pacman_render.write().unwrap().pacman_state.pause();
            }
            if playback_mode {
                if ui
                    .add_enabled(advanced_controls, icon_button("☉"))
                    .clicked()
                {
                    self.replay_manager.replay = Replay::starting_at(&self.replay_manager.replay);
                    self.mode = AppMode::Recording;
                }
            } else if ui.add_enabled(game_paused, icon_button("⏹")).clicked() {
                self.mode = AppMode::Playback;
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
                    self.replay_manager
                        .replay
                        .step_forwards_until_pacman_state();
                } else {
                    // game is live but paused
                    {
                        let mut game = self.pacman_render.write().unwrap();
                        game.pacman_state.unpause();
                        game.pacman_state.force_step();
                        game.pacman_state.pause();
                    }
                    self.replay_manager
                        .replay
                        .record_pacman_state(
                            self.pacman_render.read().unwrap().pacman_state.to_owned(),
                        )
                        .expect("Failed to record pacman state!");
                }
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏭"))
                .clicked()
                || (k_right && k_shift)
            {
                self.replay_manager.replay.go_to_end();
            }

            ui.add_enabled(
                playback_mode,
                eframe::egui::Slider::new(&mut self.replay_manager.playback_speed, -5.0..=5.0)
                    .text("Playback Speed"),
            );

            if playback_mode {
                self.update_with_replay();
            }
        });
    }

    fn update_with_replay(&mut self) {
        let mut pacman_state = self.replay_manager.replay.get_pacman_state();
        let location = self.replay_manager.replay.get_pacbot_location();

        pacman_state.pause();
        self.pacman_render.write().unwrap().pacman_state = pacman_state;

        self.replay_pacman = location.to_owned();
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

            self.mode = AppMode::Playback;
            self.replay_manager.replay = replay;
            self.update_with_replay();
            self.replay_manager.playback_paused = true;
        }

        Ok(())
    }

    pub fn reset_replay(&mut self) {
        self.replay_manager.replay = Replay::new(
            "replay".to_string(),
            self.selected_grid,
            self.pacman_render.read().unwrap().pacman_state.to_owned(),
            self.phys_render.read().unwrap().pacbot_pos,
        );
    }
}
