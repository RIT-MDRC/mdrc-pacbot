use crate::grid::standard_grids::StandardGrid;
use crate::gui::AppMode;
use crate::physics::LightPhysicsInfo;
use crate::replay::Replay;
use crate::{PacmanGameState, UserSettings};
use anyhow::Error;
use bevy::prelude::*;
use pacbot_rs::game_engine::GameEngine;
use rapier2d::math::{Rotation, Translation};
use rapier2d::na::Isometry2;
use std::time::{Duration, SystemTime};

/// The public interface for recording and replaying GUI data
#[derive(Resource)]
pub struct ReplayManager {
    /// The current replay, which may be recording or playing back
    pub replay: Replay,
    /// When current_frame was played; used to determine when to advance the replay
    pub playback_time: SystemTime,
    /// Whether playback is paused
    pub playback_paused: bool,
    /// Speed of playback - 0 is stopped, 1 is normal forwards
    pub playback_speed: f32,
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
    /// Create a new ReplayManager; assumes that it is starting in recording mode
    ///
    /// Note: pacman_state is copied once to initialize the replay
    pub fn new(
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

    fn update_with_replay(&mut self, pacman_state: &mut GameEngine) {
        let mut pacman_state_new = self.replay.get_pacman_state();

        pacman_state_new.pause();
        *pacman_state = pacman_state_new;
    }
}

/// Play back or save frames as necessary
///
/// When not in Playback mode, update_replay_playback has no effect
pub fn update_replay_manager_system(
    pacman_state: Res<PacmanGameState>,
    phys_render: Res<LightPhysicsInfo>,
    replay_manager: ResMut<ReplayManager>,
    settings: Res<UserSettings>,
) {
    if let Err(e) = update_replay_manager(pacman_state, phys_render, replay_manager, settings) {
        eprintln!("Update replay manager failed: {:?}", e);
    }
}

fn update_replay_manager(
    pacman_state: Res<PacmanGameState>,
    phys_render: Res<LightPhysicsInfo>,
    mut replay_manager: ResMut<ReplayManager>,
    settings: Res<UserSettings>,
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

    Ok(())
}

pub fn replay_playback(
    mut pacman_state: ResMut<PacmanGameState>,
    mut replay_manager: ResMut<ReplayManager>,
    settings: Res<UserSettings>,
) {
    if settings.mode == AppMode::Playback {
        replay_manager.update_with_replay(&mut pacman_state.0);
    }
}
