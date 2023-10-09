//! Records and replays GUI data

use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::PacmanState;
use crate::grid::ComputedGrid;
use crate::gui::{utils, App, AppMode, GameServer};
use crate::robot::Robot;
use crate::standard_grids::StandardGrid;
use egui::Button;
use egui::Key;
use egui::Ui;
use rand::rngs::ThreadRng;
use rapier2d::na::Isometry2;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// The types of data that might be stored in a [`ReplayFrame`]
#[derive(Clone, Serialize, Deserialize)]
pub enum ReplayFrameData {
    /// Pacbot's real physical location, as determined by the [`PacbotSimulation`]
    PacbotLocation(Isometry2<f32>),
    /// Set the current grid information
    StandardGrid(StandardGrid),
    /// Information that doesn't change throughout the game, like [`ComputedGrid`], ghost paths, etc.
    ///
    /// Encoded [`PacmanAgentSetup`]
    ///
    /// Note: this does NOT affect the current grid - for that, use the StandardGrid option, which
    /// will also reset physics
    AgentSetup(Vec<u8>),
    /// Information that changes frequently in Pacman, like ghost locations and pellets
    ///
    /// Encoded [`PacmanState`]
    PacmanGameState(Vec<u8>),
}

/// The metadata included in one frame of a [`Replay`]
#[derive(Clone, Serialize, Deserialize)]
struct ReplayFrame {
    /// The data in the frame
    pub data: ReplayFrameData,
    /// When the data was created
    pub timestamp: SystemTime,
}

/// A collection of frames representing a full replay, along with associated metadata
///
/// Often tracked through a [`ReplayManager`]
#[derive(Clone, Serialize, Deserialize)]
struct Replay {
    /// The time when recording started
    pub start_time: SystemTime,
    /// The name/label given to this replay (usually matches the file name)
    pub label: String,
    /// The data of the replay
    pub frames: Vec<ReplayFrame>,
}

impl Replay {
    /// Start a new Replay
    ///
    /// All replays should start with a [`PacmanAgentSetup`] (which includes a [`ComputedGrid`]) and
    /// a [`PacmanState`] so that everything is reset when the replay is loaded
    ///
    /// Note: agent_setup and pacman_state are copied once
    pub fn new(
        label: String,
        standard_grid: StandardGrid,
        agent_setup: &PacmanAgentSetup,
        pacman_state: &PacmanState,
        pacbot_location: Isometry2<f32>,
    ) -> Self {
        let start_time = SystemTime::now();
        let frames = vec![
            ReplayFrame {
                data: ReplayFrameData::StandardGrid(standard_grid),
                timestamp: start_time,
            },
            ReplayFrame {
                data: ReplayFrameData::AgentSetup(bincode::serialize(agent_setup).unwrap()),
                timestamp: start_time,
            },
            ReplayFrame {
                data: ReplayFrameData::PacmanGameState(bincode::serialize(pacman_state).unwrap()),
                timestamp: start_time,
            },
            ReplayFrame {
                data: ReplayFrameData::PacbotLocation(pacbot_location),
                timestamp: start_time,
            },
        ];
        Self {
            start_time,
            label,
            frames,
        }
    }
}

/// The public interface for recording and replaying GUI data
pub struct ReplayManager {
    /// The current replay, which may be recording or playing back
    replay: Replay,
    /// The current frame of the above replay
    ///
    /// When recording, this is the index of the last recorded frame, or `replay.frames.len() - 1`
    ///
    /// When in playback, this is the index of the last played frame
    current_frame: usize,
    /// When current_frame was played; used to determine when to advance the replay
    playback_time: SystemTime,
    /// Whether playback is paused
    playback_paused: bool,
    /// The file that is being saved to or loaded from
    filename: String,
}

impl App {
    /// Create a new ReplayManager; assumes that it is starting in recording mode
    ///
    /// Note: agent_setup and pacman_state are copied once to initialize the replay
    pub fn new_replay_manager(
        filename: String,
        standard_grid: StandardGrid,
        agent_setup: &PacmanAgentSetup,
        pacman_state: &PacmanState,
        pacbot_location: Isometry2<f32>,
    ) -> ReplayManager {
        let replay = Replay::new(
            filename.to_owned(),
            standard_grid,
            agent_setup,
            pacman_state,
            pacbot_location,
        );
        ReplayManager {
            current_frame: replay.frames.len() - 1,
            replay,
            playback_time: SystemTime::now(),
            playback_paused: true,
            filename,
        }
    }

    /// Play back or save frames as necessary
    ///
    /// When not in Playback mode, update_replay_playback has no effect
    ///
    /// # Returns
    ///
    /// Whether or not a new frame was played or saved
    pub fn update_replay_manager(&mut self) -> bool {
        // did pacman state request saving?
        if self.pacman_state_notify_recv.try_recv().is_ok() {
            self.record_pacman_state();
        }

        if self.mode != AppMode::Playback {
            // save physics position
            let position = self.phys_render.read().unwrap().pacbot_pos;
            self.record_pacman_location(position);
        }

        if self.replay_manager.playback_paused {
            // When playback is paused, constantly set this to now so that it starts up correctly
            self.replay_manager.playback_time = SystemTime::now();
            return false;
        }

        if self.replay_manager.current_frame == self.replay_manager.replay.frames.len() - 1 {
            self.replay_manager.playback_paused = true;
            // we have reached the end of the replay
            return false;
        }

        let now = SystemTime::now();

        if self.replay_manager.replay.frames[self.replay_manager.current_frame + 1]
            .timestamp
            .duration_since(
                self.replay_manager.replay.frames[self.replay_manager.current_frame].timestamp,
            )
            .unwrap()
            > now
                .duration_since(self.replay_manager.playback_time)
                .unwrap()
        {
            return false;
        }

        self.replay_manager.current_frame += 1;
        self.play_frame(
            &self.replay_manager.replay.frames[self.replay_manager.current_frame]
                .data
                .to_owned(),
        )
        .unwrap();
        self.replay_manager.playback_time = now;

        true
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
                self.go_to_beginning();
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏪"))
                .clicked()
                || (k_left && !k_shift)
            {
                self.step_back();
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
                    self.replay_manager
                        .replay
                        .frames
                        .truncate(self.replay_manager.current_frame + 1);
                    // update the timestamps on the frames
                    let offset = SystemTime::now()
                        .duration_since(
                            self.replay_manager.replay.frames[self.replay_manager.current_frame]
                                .timestamp,
                        )
                        .unwrap();
                    for f in &mut self.replay_manager.replay.frames {
                        f.timestamp += offset;
                    }
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
                    self.step_forwards();
                } else {
                    // game is live but paused
                    {
                        let mut game = self.pacman_render.write().unwrap();
                        game.pacman_state.resume();
                        game.pacman_state
                            .step(&self.agent_setup, &mut ThreadRng::default(), true);
                        game.pacman_state.pause();
                    }
                    self.record_pacman_state();
                }
            }
            if ui
                .add_enabled(advanced_controls, icon_button("⏭"))
                .clicked()
                || (k_right && k_shift)
            {
                self.go_to_end();
            }
        });
    }

    /// Save the data to the replay
    ///
    /// If the app is not in recording mode, this has no effect
    fn record_data(&mut self, data: ReplayFrameData) {
        if !matches!(self.mode, AppMode::Recording(_)) {
            return;
        }

        self.replay_manager.replay.frames.push(ReplayFrame {
            data,
            timestamp: SystemTime::now(),
        });
        self.replay_manager.current_frame += 1;
    }

    /// Save the physics location to the replay
    ///
    /// If the app is not in recording mode, this has no effect
    pub fn record_pacman_location(&mut self, location: Isometry2<f32>) {
        self.record_data(ReplayFrameData::PacbotLocation(location))
    }

    /// Save the standard grid to the replay
    ///
    /// If the app is not in recording mode, this has no effect
    pub fn _record_standard_grid(&mut self, standard_grid: StandardGrid) {
        self.record_data(ReplayFrameData::StandardGrid(standard_grid))
    }

    /// Save the agent setup to the replay
    ///
    /// If the app is not in recording mode, this has no effect
    pub fn _record_agent_setup(&mut self, agent_setup: &PacmanAgentSetup) {
        self.record_data(ReplayFrameData::AgentSetup(
            bincode::serialize(agent_setup).unwrap(),
        ))
    }

    /// Save the pacman state to the replay
    ///
    /// If the app is not in recording mode, this has no effect
    pub fn record_pacman_state(&mut self) {
        let bytes = bincode::serialize(&self.pacman_render.read().unwrap().pacman_state).unwrap();
        self.record_data(ReplayFrameData::PacmanGameState(bytes))
    }

    /// Use the data from the [`ReplayFrame`] to affect the GUI
    fn play_frame(&mut self, data: &ReplayFrameData) -> Result<(), bincode::Error> {
        match data {
            ReplayFrameData::PacbotLocation(location) => {
                self.replay_pacman = *location;
                Ok(())
            }
            ReplayFrameData::StandardGrid(standard_grid) => {
                self.grid = ComputedGrid::try_from(standard_grid.get_grid()).unwrap();
                self.phys_restart_send
                    .send((
                        *standard_grid,
                        Robot::default(),
                        standard_grid.get_default_pacbot_isometry(),
                    ))
                    .unwrap();
                Ok(())
            }
            ReplayFrameData::AgentSetup(data) => {
                let agent_setup = bincode::deserialize::<PacmanAgentSetup>(data)?;
                let mut pacman_render = self.pacman_render.write().unwrap();
                pacman_render.agent_setup = agent_setup.clone();
                pacman_render.pacman_state.reset(&agent_setup, true);
                self.agent_setup = agent_setup;
                Ok(())
            }
            ReplayFrameData::PacmanGameState(data) => {
                let mut pacman_state = bincode::deserialize::<PacmanState>(data)?;
                // prevent the game state thread from updating it
                pacman_state.pause();
                self.pacman_render.write().unwrap().pacman_state = pacman_state;
                Ok(())
            }
        }
    }

    /// Step backwards (until a new PacmanState is rendered)
    fn step_back(&mut self) {
        if !matches!(self.mode, AppMode::Playback) {
            return;
        }

        while self.replay_manager.current_frame > 0 {
            self.replay_manager.current_frame -= 1;

            let frame =
                self.replay_manager.replay.frames[self.replay_manager.current_frame].to_owned();
            if self.play_frame_reverse(&frame.data) {
                return;
            }
        }
    }

    /// Step forwards (until a new PacmanState is rendered)
    fn step_forwards(&mut self) {
        if !matches!(self.mode, AppMode::Playback) {
            return;
        }

        while self.replay_manager.current_frame + 1 < self.replay_manager.replay.frames.len() {
            self.replay_manager.current_frame += 1;

            let frame =
                self.replay_manager.replay.frames[self.replay_manager.current_frame].to_owned();
            // play each one
            self.play_frame(&frame.data).unwrap();
            if matches!(frame.data, ReplayFrameData::PacmanGameState(_)) {
                return;
            }
        }
    }

    /// Go back to the beginning
    fn go_to_beginning(&mut self) {
        if !matches!(self.mode, AppMode::Playback) {
            return;
        }

        let beginning_time = self.replay_manager.replay.frames[0].timestamp;
        self.replay_manager.current_frame = 0;
        let data = &self.replay_manager.replay.frames[0].data.to_owned();
        self.play_frame_reverse(data);

        while self.replay_manager.current_frame + 1 < self.replay_manager.replay.frames.len()
            && self.replay_manager.replay.frames[self.replay_manager.current_frame + 1].timestamp
                == beginning_time
        {
            self.replay_manager.current_frame += 1;
            let data = &self.replay_manager.replay.frames[self.replay_manager.current_frame]
                .data
                .to_owned();
            self.play_frame_reverse(data);
        }
    }

    /// Go to the end
    fn go_to_end(&mut self) {
        if !matches!(self.mode, AppMode::Playback) {
            return;
        }

        while self.replay_manager.current_frame + 1 < self.replay_manager.replay.frames.len() {
            self.replay_manager.current_frame += 1;
            let data = &self.replay_manager.replay.frames[self.replay_manager.current_frame]
                .data
                .to_owned();
            self.play_frame(data).unwrap();
        }
    }

    /// Play frames that make sense in reverse; return true if it was a game state
    fn play_frame_reverse(&mut self, data: &ReplayFrameData) -> bool {
        match data {
            ReplayFrameData::PacbotLocation(_) => {
                self.play_frame(data).unwrap();
            }
            ReplayFrameData::PacmanGameState(_) => {
                self.play_frame(data).unwrap();
                return true;
            }
            _ => {}
        }

        false
    }
}
