//! Records and replays GUI data

use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::PacmanState;
use crate::gui::{utils, App, AppMode, GameServer};
use crate::replay::Replay;
use crate::standard_grids::StandardGrid;
use anyhow::Error;
use egui::Button;
use egui::Key;
use egui::Ui;
use rand::rngs::ThreadRng;
use rapier2d::na::Isometry2;
use std::time::SystemTime;

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

impl App {
    /// Create a new ReplayManager; assumes that it is starting in recording mode
    ///
    /// Note: agent_setup and pacman_state are copied once to initialize the replay
    pub fn new_replay_manager(
        filename: String,
        standard_grid: StandardGrid,
        agent_setup: PacmanAgentSetup,
        pacman_state: &PacmanState,
        pacbot_location: Isometry2<f32>,
    ) -> ReplayManager {
        let replay = Replay::new(
            filename.to_owned(),
            standard_grid,
            agent_setup,
            pacman_state,
            pacbot_location,
        )
        .expect("Failed to create new replay!");
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
    ///
    /// # Returns
    ///
    /// Whether or not a new frame was played or saved
    pub fn update_replay_manager(&mut self) -> Result<bool, Error> {
        // did pacman state request saving?
        if self.pacman_state_notify_recv.try_recv().is_ok() {
            self.replay_manager
                .replay
                .record_pacman_state(&self.pacman_render.read().unwrap().pacman_state)?;
        }

        if self.mode != AppMode::Playback {
            // save physics position
            let position = self.phys_render.read().unwrap().pacbot_pos;
            // If it is exactly (0,0), then it hasn't been initialized yet
            self.replay_manager
                .replay
                .record_pacman_location(position)?;
        }

        if self.replay_manager.playback_paused {
            // When playback is paused, constantly set this to now so that it starts up correctly
            self.replay_manager.playback_time = SystemTime::now();
            return Ok(false);
        }

        let now = SystemTime::now();

        let should_step_replay = match self.replay_manager.replay.get_next() {
            None => {
                self.replay_manager.playback_paused = true;
                // we have reached the end of the replay
                false
            }
            Some(next) => {
                next.timestamp
                    .duration_since(self.replay_manager.replay.get().timestamp)?
                    .as_secs_f32()
                    < now
                        .duration_since(self.replay_manager.playback_time)?
                        .as_secs_f32()
                        * self.replay_manager.playback_speed
            }
        };

        if should_step_replay {
            self.replay_manager.replay.step_forwards();
        }

        self.update_with_replay()?;
        self.replay_manager.playback_time = now;

        Ok(true)
    }

    /// Draw the UI involved in recording/playback
    ///
    /// ui should be just the bottom panel
    pub fn draw_replay_ui(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let game_paused = self.pacman_render.write().unwrap().pacman_state.paused;

        let k_space = ctx.input(|i| i.key_pressed(Key::Space));
        let k_left = ctx.input(|i| i.key_pressed(Key::ArrowLeft));
        let k_right = ctx.input(|i| i.key_pressed(Key::ArrowRight));
        let k_shift = ctx.input(|i| i.modifiers.shift);

        utils::centered_group(ui, |ui| {
            let icon_button_size = egui::vec2(22.0, 22.0);
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
                    self.pacman_render.write().unwrap().pacman_state.resume();
                }
            } else if ui.add_enabled(true, icon_button("⏸")).clicked() || k_space {
                self.pacman_render.write().unwrap().pacman_state.pause();
            }
            if playback_mode {
                if ui
                    .add_enabled(advanced_controls, icon_button("☉"))
                    .clicked()
                {
                    self.replay_manager.replay = Replay::starting_at(&self.replay_manager.replay)
                        .expect("Failed to create new replay");
                    self.mode = AppMode::Recording(GameServer::Simulated);
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
                        game.pacman_state.resume();
                        game.pacman_state
                            .step(&self.agent_setup, &mut ThreadRng::default(), true);
                        game.pacman_state.pause();
                    }
                    self.replay_manager
                        .replay
                        .record_pacman_state(&self.pacman_render.read().unwrap().pacman_state)
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
                egui::Slider::new(&mut self.replay_manager.playback_speed, 0.0..=5.0)
                    .text("Playback Speed"),
            );

            if playback_mode {
                self.update_with_replay()
                    .expect("Failed to update using replay variables");
            }
        });
    }

    fn update_with_replay(&mut self) -> Result<(), Error> {
        let mut pacman_state = self.replay_manager.replay.get_pacman_state()?;
        let location = self.replay_manager.replay.get_pacbot_location()?;

        pacman_state.pause();
        self.pacman_render.write().unwrap().pacman_state = pacman_state;

        self.replay_pacman = location.to_owned();

        Ok(())
    }
}
