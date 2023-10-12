//! A utility for recording over time

use crate::agent_setup::PacmanAgentSetup;
use crate::game_state::PacmanState;
use crate::standard_grids::StandardGrid;
use anyhow::{anyhow, Error};
use rapier2d::na::Isometry2;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// The types of data that might be stored in a [`ReplayFrame`]
#[derive(Clone, Serialize, Deserialize)]
enum ReplayFrameData {
    /// Pacbot's real physical location, as determined by the [`PacbotSimulation`]
    PacbotLocation(Isometry2<f32>),
    /// Information that changes frequently in Pacman, like ghost locations and pellets
    PacmanGameState(Box<PacmanState>),
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

impl Default for Replay {
    fn default() -> Self {
        Self::new(
            "replay".to_string(),
            StandardGrid::Pacman,
            PacmanAgentSetup::default(),
            PacmanState::default(),
            StandardGrid::Pacman.get_default_pacbot_isometry(),
        )
    }
}

impl Replay {
    /// Start a new Replay
    ///
    /// Note: pacman_state is copied once
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::math::Isometry;
    /// use rapier2d::na::Vector2;
    /// use mdrc_pacbot_util::agent_setup::PacmanAgentSetup;
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    /// use mdrc_pacbot_util::standard_grids::StandardGrid;
    ///
    /// let mut replay = Replay::new(
    ///     "My First Replay".to_string(),
    ///     StandardGrid::Pacman,
    ///     PacmanAgentSetup::default(),
    ///     PacmanState::default(),
    ///     StandardGrid::Pacman.get_default_pacbot_isometry()
    /// );
    ///
    /// assert_eq!(replay.frame_count(), 2);
    /// assert_eq!(replay.current_frame(), 1);
    /// ```
    pub fn new(
        label: String,
        standard_grid: StandardGrid,
        agent_setup: PacmanAgentSetup,
        pacman_state: PacmanState,
        pacbot_location: Isometry2<f32>,
    ) -> Self {
        let start_time = SystemTime::now();
        let frames = vec![
            ReplayFrame {
                data: ReplayFrameData::PacmanGameState(Box::new(pacman_state)),
                timestamp: start_time,
            },
            ReplayFrame {
                data: ReplayFrameData::PacbotLocation(pacbot_location),
                timestamp: start_time,
            },
        ];
        Self {
            start_time,
            standard_grid,
            agent_setup,
            label,
            frames,
            current_frame: 1,
            pacman_state_frame: 0,
            location_frame: 1,
        }
    }

    /// Create a new Replay starting at the current frame in the given Replay
    ///
    /// All frames are updated so that the most recent one matches the current time
    ///
    /// May return Err if the given replay frame is in the future
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::rngs::ThreadRng;
    /// use rapier2d::math::Isometry;
    /// use rapier2d::na::Vector2;
    /// use mdrc_pacbot_util::agent_setup::PacmanAgentSetup;
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    /// use mdrc_pacbot_util::standard_grids::StandardGrid;
    ///
    /// let agent_setup = PacmanAgentSetup::default();
    /// let mut pacman_state = PacmanState::new(&agent_setup);
    /// let mut rng = ThreadRng::default();
    ///
    /// let mut replay = Replay::new(
    ///     "My First Replay".to_string(),
    ///     StandardGrid::Pacman,
    ///     PacmanAgentSetup::default(),
    ///     pacman_state.to_owned(),
    ///     StandardGrid::Pacman.get_default_pacbot_isometry()
    /// );
    ///
    /// assert_eq!(replay.frame_count(), 2);
    /// assert_eq!(replay.current_frame(), 1);
    ///
    /// for i in 0..3 {
    ///     pacman_state.step(&agent_setup, &mut rng, false);
    ///     replay.record_pacman_state(pacman_state.to_owned()).unwrap();
    /// }
    ///
    /// replay.step_back();
    /// replay.step_back();
    ///
    /// assert!(!replay.is_at_end());
    /// assert_eq!(replay.frame_count(), 5);
    /// assert_eq!(replay.current_frame(), 2);
    ///
    /// let replay = Replay::starting_at(&replay);
    ///
    /// assert!(replay.is_at_end());
    /// assert_eq!(replay.frame_count(), 3);
    /// assert_eq!(replay.current_frame(), 2);
    /// ```
    pub fn starting_at(other: &Replay) -> Self {
        let mut frames = Vec::new();
        let now = SystemTime::now();
        let offset = now
            .duration_since(other.frames[other.current_frame].timestamp)
            .expect("Other replay ended in the future!");

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

        Self {
            start_time: frames[0].timestamp,
            standard_grid: other.standard_grid,
            agent_setup: other.agent_setup.to_owned(),
            label: other.label.to_owned(),
            frames,
            current_frame: other.current_frame,
            pacman_state_frame,
            location_frame,
        }
    }

    /// Create a new Replay using bytes from a file
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let replay = Replay::default();
    /// let replay_bytes = replay.to_bytes().expect("Failed to serialize replay!");
    /// let replay2 = Replay::from_bytes(&replay_bytes).expect("Failed to deserialize replay!");
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Replay, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Get the bytes associated with the Replay
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let replay = Replay::default();
    /// let replay_bytes = replay.to_bytes().expect("Failed to serialize replay!");
    /// let replay2 = Replay::from_bytes(&replay_bytes).expect("Failed to deserialize replay!");
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Returns whether the replay has played its last frame
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    /// assert!(replay.is_at_end());
    /// replay.record_pacman_state(PacmanState::default()).unwrap();
    /// assert!(replay.is_at_end());
    /// replay.step_back();
    /// assert!(!replay.is_at_end());
    /// replay.step_forwards();
    /// assert!(replay.is_at_end());
    /// ```
    pub fn is_at_end(&self) -> bool {
        self.current_frame == self.frames.len() - 1
    }

    /// Returns whether the replay is at the beginning
    ///
    /// # Examples
    ///
    /// ```
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    /// assert!(replay.is_at_beginning());
    /// replay.record_pacman_state(PacmanState::default()).unwrap();
    /// assert!(!replay.is_at_beginning());
    /// replay.step_back();
    /// assert!(replay.is_at_beginning());
    /// replay.step_forwards();
    /// assert!(!replay.is_at_beginning());
    /// ```
    pub fn is_at_beginning(&self) -> bool {
        self.current_frame <= 1
    }

    /// Returns the most recent PacmanState
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::rngs::ThreadRng;
    /// use mdrc_pacbot_util::agent_setup::PacmanAgentSetup;
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    /// let mut pacman_state = PacmanState::default();
    /// let mut agent_setup = PacmanAgentSetup::default();
    /// let mut rng = ThreadRng::default();
    ///
    /// pacman_state.step(&agent_setup, &mut rng, false);
    /// replay.record_pacman_state(pacman_state.to_owned()).unwrap();
    /// let pacman_state_1 = pacman_state.to_owned();
    /// pacman_state.step(&agent_setup, &mut rng, false);
    /// replay.record_pacman_state(pacman_state.to_owned()).unwrap();
    /// let pacman_state_2 = pacman_state.to_owned();
    ///
    /// assert_eq!(pacman_state_2, replay.get_pacman_state());
    /// replay.step_backwards_until_pacman_state();
    /// assert_eq!(pacman_state_1, replay.get_pacman_state());
    /// replay.step_backwards_until_pacman_state();
    /// assert_eq!(PacmanState::default(), replay.get_pacman_state());
    /// ```
    pub fn get_pacman_state(&self) -> PacmanState {
        if let ReplayFrameData::PacmanGameState(data) = &self.frames[self.pacman_state_frame].data {
            *data.to_owned()
        } else {
            panic!("Replay was corrupt - pacman_state_frame was wrong")
        }
    }

    /// Returns the most recent PacbotLocation
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::{Isometry2, Vector2};
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    ///
    /// let isometry_1 = Isometry2::new(Vector2::new(1.0, 1.0), 1.0);
    /// replay.record_pacman_location(isometry_1.to_owned()).unwrap();
    /// let isometry_2 = Isometry2::new(Vector2::new(2.0, 2.0), 2.0);
    /// replay.record_pacman_location(isometry_2.to_owned()).unwrap();
    ///
    /// assert_eq!(isometry_2, replay.get_pacbot_location());
    /// replay.step_back();
    /// assert_eq!(isometry_1, replay.get_pacbot_location());
    /// ```
    pub fn get_pacbot_location(&self) -> Isometry2<f32> {
        if let ReplayFrameData::PacbotLocation(data) = &self.frames[self.location_frame].data {
            data.to_owned()
        } else {
            panic!("Replay was corrupt - location_frame was wrong")
        }
    }

    fn update_current_frames(&mut self) {
        match &self.frames[self.current_frame].data {
            ReplayFrameData::PacbotLocation(_) => self.location_frame = self.current_frame,
            ReplayFrameData::PacmanGameState(_) => self.pacman_state_frame = self.current_frame,
        };
    }

    /// Moves to the next frame, if it exists
    pub fn step_forwards(&mut self) {
        if !self.is_at_end() {
            self.current_frame += 1;
            self.update_current_frames();
        }
    }

    /// Moves to the previous frame, if it exists
    pub fn step_back(&mut self) {
        if self.current_frame >= 2 {
            self.current_frame -= 1;
            self.update_current_frames();
        } else {
            // play all the frames from the beginning
            self.go_to_beginning();
        }
    }

    /// Go back to the beginning of the recording
    pub fn go_to_beginning(&mut self) {
        self.current_frame = 0;
        self.pacman_state_frame = 0;
        self.location_frame = 1;
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
    ///
    /// # Examples
    ///
    /// ```
    /// use rapier2d::na::{Isometry2, Vector2};
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    ///
    /// replay.record_pacman_state(PacmanState::default()).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(1.0, 1.0), 1.0)).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(2.0, 2.0), 2.0)).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(3.0, 3.0), 3.0)).unwrap();
    /// replay.record_pacman_state(PacmanState::default()).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(4.0, 4.0), 4.0)).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(5.0, 5.0), 5.0)).unwrap();
    /// replay.record_pacman_location(Isometry2::new(Vector2::new(6.0, 6.0), 6.0)).unwrap();
    /// replay.record_pacman_state(PacmanState::default()).unwrap();
    ///
    /// assert_eq!(replay.get_pacbot_location(), Isometry2::new(Vector2::new(6.0, 6.0), 6.0));
    /// replay.step_backwards_until_pacman_state();
    /// assert_eq!(replay.get_pacbot_location(), Isometry2::new(Vector2::new(4.0, 4.0), 4.0));
    /// replay.step_backwards_until_pacman_state();
    /// assert_eq!(replay.get_pacbot_location(), Isometry2::new(Vector2::new(1.0, 1.0), 1.0));
    /// replay.step_forwards_until_pacman_state();
    /// assert_eq!(replay.get_pacbot_location(), Isometry2::new(Vector2::new(3.0, 3.0), 3.0));
    /// replay.step_forwards_until_pacman_state();
    /// assert_eq!(replay.get_pacbot_location(), Isometry2::new(Vector2::new(6.0, 6.0), 6.0));
    /// ```
    pub fn step_forwards_until_pacman_state(&mut self) {
        let previous_pacman_state_frame = self.pacman_state_frame;
        while !self.is_at_end() {
            self.step_forwards();
            if previous_pacman_state_frame != self.pacman_state_frame {
                return;
            }
        }
    }

    /// Step backwards until a PacmanState frame is reached
    ///
    /// See [`Replay::step_forwards_until_pacman_state`] for example
    pub fn step_backwards_until_pacman_state(&mut self) {
        let previous_pacman_state_frame = self.pacman_state_frame;
        while !self.is_at_beginning() {
            self.step_back();
            if previous_pacman_state_frame != self.pacman_state_frame {
                return;
            }
        }
    }

    /// Add a pacman location to the end of the replay
    ///
    /// Returns err if the current frame is not the last frame
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// use mdrc_pacbot_util::game_state::PacmanState;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    ///
    /// assert!(replay.record_pacman_state(PacmanState::default()).is_ok());
    /// replay.step_back();
    /// // Don't add frames to a replay that isn't at the end
    /// assert!(replay.record_pacman_state(PacmanState::default()).is_err());
    /// ```
    pub fn record_pacman_location(&mut self, location: Isometry2<f32>) -> Result<(), Error> {
        if !self.is_at_end() {
            Err(anyhow!("Tried to record to replay that was mid-playback"))
        } else {
            self.frames.push(ReplayFrame {
                timestamp: SystemTime::now(),
                data: ReplayFrameData::PacbotLocation(location),
            });
            self.current_frame += 1;
            self.location_frame = self.current_frame;
            Ok(())
        }
    }

    /// Add a pacman game state to the end of the replay
    ///
    /// Returns err if the current frame is not the last frame
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// use rapier2d::na::Isometry2;
    /// use mdrc_pacbot_util::replay::Replay;
    ///
    /// let mut replay = Replay::default();
    ///
    /// assert!(replay.record_pacman_location(Isometry2::default()).is_ok());
    /// replay.step_back();
    /// // Don't add frames to a replay that isn't at the end
    /// assert!(replay.record_pacman_location(Isometry2::default()).is_err());
    /// ```
    pub fn record_pacman_state(&mut self, state: PacmanState) -> Result<(), Error> {
        if !self.is_at_end() {
            Err(anyhow!("Tried to record to replay that was mid-playback"))
        } else {
            self.frames.push(ReplayFrame {
                timestamp: SystemTime::now(),
                data: ReplayFrameData::PacmanGameState(Box::new(state)),
            });
            self.current_frame += 1;
            self.pacman_state_frame = self.current_frame;
            Ok(())
        }
    }

    /// Get the index of the current frame
    ///
    /// While this can be a good estimation of the progress through a replay, there is no guarantee
    /// that step_forwards() or step_back(), or any other methods, will hit any specific frame
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Get the number of frames
    ///
    /// While this can be used to get a good estimation of the progress through a replay, there
    /// is no guarantee that step_forwards() or step_back(), or any other methods, will hit
    /// any specific frame
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the amount of time until the next frame
    ///
    /// If at the end, returns Duration::MAX
    pub fn time_to_next(&self) -> Duration {
        if self.is_at_end() {
            Duration::MAX
        } else {
            self.frames[self.current_frame + 1]
                .timestamp
                .duration_since(self.frames[self.current_frame].timestamp)
                .expect("Frames are out of order!")
        }
    }

    /// Get the amount of time between the current and previous frame
    ///
    /// If at the end, returns Duration::MAX
    pub fn time_to_previous(&self) -> Duration {
        if self.is_at_beginning() {
            Duration::MAX
        } else {
            self.frames[self.current_frame]
                .timestamp
                .duration_since(self.frames[self.current_frame - 1].timestamp)
                .expect("Frames are out of order!")
        }
    }
}

#[cfg(test)]
mod test {
    use crate::game_state::PacmanState;
    use crate::replay::Replay;
    use rapier2d::na::Isometry2;

    #[test]
    fn test_replay_starting_at_end_of_other() {
        let mut replay = Replay::default();

        replay.record_pacman_location(Isometry2::default()).unwrap();
        replay.record_pacman_location(Isometry2::default()).unwrap();
        replay.record_pacman_state(PacmanState::default()).unwrap();
        replay.record_pacman_location(Isometry2::default()).unwrap();
        replay.record_pacman_location(Isometry2::default()).unwrap();

        let replay = Replay::starting_at(&replay);

        replay.get_pacman_state();
        replay.get_pacbot_location();
    }
}
