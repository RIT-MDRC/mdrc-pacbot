//! A utility for recording over time

use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::PacmanState;
use crate::standard_grids::StandardGrid;
use anyhow::{anyhow, Error};
use rapier2d::na::Isometry2;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// The types of data that might be stored in a [`ReplayFrame`]
#[derive(Clone, Serialize, Deserialize)]
pub enum ReplayFrameData {
    /// Pacbot's real physical location, as determined by the [`PacbotSimulation`]
    PacbotLocation(Isometry2<f32>),
    /// Information that changes frequently in Pacman, like ghost locations and pellets
    ///
    /// Encoded [`PacmanState`]
    PacmanGameState(Vec<u8>),
}

/// The metadata included in one frame of a [`Replay`]
#[derive(Clone, Serialize, Deserialize)]
pub struct ReplayFrame {
    /// The data in the frame
    pub data: ReplayFrameData,
    /// When the data was created
    pub timestamp: SystemTime,
}

/// A collection of frames representing a full replay, along with associated metadata
#[derive(Clone, Serialize, Deserialize)]
pub struct Replay {
    /// The time when recording started
    start_time: SystemTime,
    /// The StandardGrid the recording uses
    standard_grid: StandardGrid,
    /// The agent setup the recording uses
    agent_setup: PacmanAgentSetup,
    /// The name/label given to this replay (usually matches the file name)
    pub label: String,
    /// The data of the replay
    frames: Vec<ReplayFrame>,
    /// Index of the most recently recorded or played frame
    current_frame: usize,
    /// Index of the most recently recorded or played pacman state frame
    pacman_state_frame: usize,
    /// Index of the most recently recorded or played pacman location frame
    location_frame: usize,
}

impl Replay {
    /// Start a new Replay
    ///
    /// Note: pacman_state is copied once
    pub fn new(
        label: String,
        standard_grid: StandardGrid,
        agent_setup: PacmanAgentSetup,
        pacman_state: &PacmanState,
        pacbot_location: Isometry2<f32>,
    ) -> Result<Self, bincode::Error> {
        let start_time = SystemTime::now();
        let frames = vec![
            ReplayFrame {
                data: ReplayFrameData::PacmanGameState(bincode::serialize(pacman_state)?),
                timestamp: start_time,
            },
            ReplayFrame {
                data: ReplayFrameData::PacbotLocation(pacbot_location),
                timestamp: start_time,
            },
        ];
        Ok(Self {
            start_time,
            standard_grid,
            agent_setup,
            label,
            frames,
            current_frame: 1,
            pacman_state_frame: 0,
            location_frame: 1,
        })
    }

    /// Create a new Replay starting at the current frame in the given Replay
    ///
    /// All frames are updated so that the most recent one matches the current time
    ///
    /// May return Err if the given replay frame is in the future
    pub fn starting_at(other: &Replay) -> Result<Replay, Error> {
        let mut frames = Vec::new();
        let now = SystemTime::now();
        let offset = now.duration_since(other.frames[other.current_frame].timestamp)?;

        let mut pacman_state_frame = 0;
        let mut location_frame = 0;

        for frame in 0..=other.current_frame {
            match other.frames[frame].data {
                ReplayFrameData::PacbotLocation(_) => location_frame = frame,
                ReplayFrameData::PacmanGameState(_) => pacman_state_frame = frame,
            }
            frames.push(ReplayFrame {
                data: other.frames[frame].data.to_owned(),
                timestamp: other.frames[frame].timestamp + offset,
            })
        }

        println!("new: {} {}", frames.len(), other.location_frame);

        Ok(Self {
            start_time: frames[0].timestamp,
            standard_grid: other.standard_grid,
            agent_setup: other.agent_setup.to_owned(),
            label: other.label.to_owned(),
            frames,
            current_frame: other.current_frame,
            pacman_state_frame,
            location_frame,
        })
    }

    /// Create a new Replay using bytes from a file
    pub fn from_bytes(bytes: &[u8]) -> Result<Replay, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Get the bytes associated with the Replay
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Returns whether the replay has played its last frame
    pub fn is_ended(&self) -> bool {
        self.current_frame == self.frames.len() - 1
    }

    /// Returns the current frame
    pub fn get(&self) -> &ReplayFrame {
        &self.frames[self.current_frame]
    }

    /// Returns the next frame, if it exists
    pub fn get_next(&self) -> Option<&ReplayFrame> {
        if self.is_ended() {
            None
        } else {
            Some(&self.frames[self.current_frame + 1])
        }
    }

    /// Returns the most recent PacmanState
    pub fn get_pacman_state(&self) -> Result<PacmanState, Error> {
        if let ReplayFrameData::PacmanGameState(data) = &self.frames[self.pacman_state_frame].data {
            let pacman_state: PacmanState = bincode::deserialize(data)?;
            Ok(pacman_state)
        } else {
            Err(anyhow!("Replay was corrupt - pacman_state_frame was wrong"))
        }
    }

    /// Returns the most recent PacbotLocation
    pub fn get_pacbot_location(&self) -> Result<Isometry2<f32>, Error> {
        if let ReplayFrameData::PacbotLocation(data) = &self.frames[self.location_frame].data {
            Ok(data.to_owned())
        } else {
            Err(anyhow!("Replay was corrupt - location_frame was wrong"))
        }
    }

    fn update_current_frames(&mut self) {
        match &self.frames[self.current_frame].data {
            ReplayFrameData::PacbotLocation(_) => self.location_frame = self.current_frame,
            ReplayFrameData::PacmanGameState(_) => self.pacman_state_frame = self.current_frame,
        };
    }

    /// Moves to the next frame and returns it
    pub fn step_forwards(&mut self) -> Option<&ReplayFrame> {
        if self.is_ended() {
            None
        } else {
            self.current_frame += 1;
            self.update_current_frames();
            Some(&self.frames[self.current_frame])
        }
    }

    /// Moves to the previous frame and returns it
    pub fn step_back(&mut self) -> Option<&ReplayFrame> {
        if self.current_frame == 0 {
            None
        } else {
            self.current_frame -= 1;
            self.update_current_frames();
            Some(&self.frames[self.current_frame])
        }
    }

    /// Go back to the beginning of the recording
    pub fn go_to_beginning(&mut self) {
        self.current_frame = 0;
        self.update_current_frames();
    }

    /// Go to the end of the recording
    pub fn go_to_end(&mut self) {
        for frame in 0..self.frames.len() {
            match &self.frames[frame].data {
                ReplayFrameData::PacbotLocation(_) => self.location_frame = frame,
                ReplayFrameData::PacmanGameState(_) => self.pacman_state_frame = frame,
            }
        }

        self.current_frame = self.frames.len() - 1;
    }

    /// Step forwards until a PacmanState frame is reached
    pub fn step_forwards_until_pacman_state(&mut self) {
        let previous_pacman_state_frame = self.pacman_state_frame;
        while self.step_forwards().is_some() {
            if previous_pacman_state_frame != self.pacman_state_frame {
                return;
            }
        }
    }

    /// Step backwards until a PacmanState frame is reached
    pub fn step_backwards_until_pacman_state(&mut self) {
        let previous_pacman_state_frame = self.pacman_state_frame;
        while self.step_back().is_some() {
            if previous_pacman_state_frame != self.pacman_state_frame {
                return;
            }
        }
    }

    /// Add a pacman location to the end of the replay
    ///
    /// Returns err if the current frame is not the last frame
    pub fn record_pacman_location(&mut self, location: Isometry2<f32>) -> Result<(), Error> {
        if !self.is_ended() {
            Err(anyhow!("Tried to record to replay that was mid-playback"))
        } else {
            self.frames.push(ReplayFrame {
                timestamp: SystemTime::now(),
                data: ReplayFrameData::PacbotLocation(location),
            });
            self.current_frame += 1;
            self.location_frame = self.current_frame;
            println!("record: {}", self.frames.len());
            Ok(())
        }
    }

    /// Add a pacman game state to the end of the replay
    ///
    /// Returns err if the current frame is not the last frame, or if serialization failed
    pub fn record_pacman_state(&mut self, state: &PacmanState) -> Result<(), Error> {
        if !self.is_ended() {
            Err(anyhow!("Tried to record to replay that was mid-playback"))
        } else {
            match bincode::serialize(state) {
                Ok(state) => {
                    self.frames.push(ReplayFrame {
                        timestamp: SystemTime::now(),
                        data: ReplayFrameData::PacmanGameState(state),
                    });
                    self.current_frame += 1;
                    self.pacman_state_frame = self.current_frame;
                    Ok(())
                }
                Err(x) => Err(anyhow!(format!("Serialization error: {}", x.to_string()))),
            }
        }
    }

    /// Get the index of the current frame
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Get the number of frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}
