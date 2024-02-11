//! Records and replays GUI data

use crate::grid::standard_grids::StandardGrid;
use crate::gui::{utils, AppMode, TabViewer};
use crate::physics::LightPhysicsInfo;
use crate::replay::Replay;
use crate::{PacmanGameState, UserSettings};
use anyhow::Error;
use bevy::prelude::*;
use eframe::egui::Button;
use eframe::egui::Key;
use eframe::egui::Ui;
use native_dialog::FileDialog;
use pacbot_rs::game_engine::GameEngine;
use rapier2d::math::Rotation;
use rapier2d::na::Isometry2;
use rapier2d::prelude::Translation;
use std::cell::RefMut;
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

impl Default for ReplayManager {
    fn default() -> Self {
        Self {
            replay: Replay::default(),
            playback_time: SystemTime::now(),
            playback_paused: true,
            playback_speed: 1.0,
        }
    }
}

impl ReplayManager {
    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    pub fn reset_replay(
        &mut self,
        selected_grid: StandardGrid,
        pacman_state: &GameEngine,
        pacbot_pos: Isometry2<f32>,
    ) {
        self.replay = Replay::new(
            "replay".to_string(),
            selected_grid,
            pacman_state.to_owned(),
            pacbot_pos,
        );
    }
}

impl<'a> TabViewer<'a> {
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
    pub fn update_replay_manager(
        &mut self,
        pacman_state: Ref<PacmanGameState>,
        phys_render: Ref<LightPhysicsInfo>,
        mut replay_manager: RefMut<ReplayManager>,
        settings: Ref<UserSettings>,
    ) -> Result<(), Error> {
        if pacman_state.is_changed() && settings.mode != AppMode::Playback {
            // if we aren't recording the physics position, we should record the game position
            if !settings.replay_save_location {
                replay_manager
                    .replay
                    .record_pacman_location(Isometry2::from_parts(
                        Translation::new(
                            pacman_state.0.get_state().pacman_loc.row as f32,
                            pacman_state.0.get_state().pacman_loc.col as f32,
                        ),
                        Rotation::new(match pacman_state.0.get_state().pacman_loc.dir {
                            pacbot_rs::location::RIGHT => std::f32::consts::FRAC_PI_2,
                            pacbot_rs::location::UP => std::f32::consts::PI,
                            pacbot_rs::location::LEFT => std::f32::consts::FRAC_PI_2 * 3.0,
                            pacbot_rs::location::DOWN => 0.0,
                            _ => 0.0,
                        }),
                    ))?;
            }
            replay_manager.replay.record_pacman_state(&pacman_state.0)?;
        }

        if settings.mode != AppMode::Playback && settings.replay_save_location {
            // save physics position
            if let Some(position) = phys_render.real_pos {
                replay_manager.replay.record_pacman_location(position)?;
            }
        }

        if replay_manager.playback_paused {
            // When playback is paused, constantly set this to now so that it starts up correctly
            replay_manager.playback_time = SystemTime::now();
            return Ok(());
        }

        if replay_manager.replay.is_at_end() {
            replay_manager.playback_paused = true;
            // we have reached the end of the replay
            return Ok(());
        }

        let now = SystemTime::now();

        if replay_manager.playback_speed >= 0.0 {
            loop {
                let time_to_next = replay_manager.replay.time_to_next().as_secs_f32();
                let should_step_replay = time_to_next / replay_manager.playback_speed
                    < now
                        .duration_since(replay_manager.playback_time)?
                        .as_secs_f32();

                if !should_step_replay {
                    break;
                }

                replay_manager.replay.step_forwards();
                let speed = replay_manager.playback_speed;
                replay_manager.playback_time += Duration::from_secs_f32(time_to_next / speed);
            }
        } else {
            loop {
                let time_to_previous = replay_manager.replay.time_to_previous().as_secs_f32();
                let should_step_replay = time_to_previous / -replay_manager.playback_speed
                    < now
                        .duration_since(replay_manager.playback_time)?
                        .as_secs_f32();

                if !should_step_replay {
                    break;
                }

                replay_manager.replay.step_back();
                let speed = replay_manager.playback_speed;
                replay_manager.playback_time += Duration::from_secs_f32(time_to_previous / -speed);
            }
        }

        // TODO
        // self.update_with_replay(&mut pacman_state.0, &replay_manager);

        Ok(())
    }

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

            if playback_mode {
                self.update_with_replay(pacman_state, replay_manager);
            }
        });
    }

    fn update_with_replay(
        &mut self,
        pacman_state: &mut GameEngine,
        replay_manager: &ReplayManager,
    ) {
        let mut pacman_state_new = replay_manager.replay.get_pacman_state();

        pacman_state_new.pause();
        *pacman_state = pacman_state_new;
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
        game_state: &mut GameEngine,
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
            self.update_with_replay(game_state, &replay_manager);
            replay_manager.playback_paused = true;
        }

        Ok(())
    }
}
