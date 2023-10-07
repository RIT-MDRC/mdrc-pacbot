//! Records and replays GUI data

use crate::gui::{PacmanStateRenderInfo, PhysicsRenderInfo, ReplayRenderInfo};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::SystemTime;

/// Commands available to tell [`ReplayManager`] what to do
pub enum ReplayManagerCommand {
    /// Start recording to the given file
    Record(String),
    /// Load the given file for playback
    Playback(String),
    /// Go back to the beginning of the recording
    Beginning,
    /// Step back one step
    StepBack,
    /// Step forward one step
    StepForward,
    /// Go to the end of the recording
    End,

    /// Record a frame of physics information associated with the given time
    RecordPhys(SystemTime),
    /// Record a frame of pacman information associated with the given time
    RecordPacman(SystemTime),
}

/// The types of things that can be recorded
#[derive(Serialize, Deserialize, PartialEq, Copy, Clone)]
enum RecordType {
    /// physics information
    PhysRender,
    /// pacman information
    PacmanRender,
}

/// The metadata included in one frame
#[derive(Serialize, Deserialize)]
struct ReplayFrame {
    /// The type of the frame
    record: RecordType,
    /// The data in the frame
    data: Vec<u8>,
    /// When the data was created
    timestamp: SystemTime,
}

/// Records and replays GUI data
pub struct ReplayManager {
    replay_render: Arc<RwLock<ReplayRenderInfo>>,
    /// Channel where commands are received
    commands: Receiver<ReplayManagerCommand>,

    /// Stores state needed to render physics information
    phys_render: Arc<RwLock<PhysicsRenderInfo>>,
    /// Stores state needed to render game state information
    pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,

    /// Stores the recording
    frames: Vec<ReplayFrame>,
    /// Which frame within frames is the current one
    ///
    /// For recording, this is the index of the last recorded frame
    ///
    /// For playback, this is the index of the last played frame
    current_frame: usize,
}

impl ReplayManager {
    /// Creates a new ReplayManager
    pub fn new(
        phys_render: Arc<RwLock<PhysicsRenderInfo>>,
        pacman_render: Arc<RwLock<PacmanStateRenderInfo>>,
        replay_render: Arc<RwLock<ReplayRenderInfo>>,

        commands: Receiver<ReplayManagerCommand>,
    ) -> ReplayManager {
        let frames = vec![];

        let mut s = Self {
            replay_render,

            commands,

            phys_render,
            pacman_render,

            frames,
            current_frame: 0,
        };

        // initial state
        // s.record_frame(RecordType::PhysRender, SystemTime::now());
        s.record_frame(RecordType::PacmanRender, SystemTime::now(), None);

        // current_frame should be the index of the last recorded frame
        s.current_frame -= 1;

        s
    }

    /// Run the replay manager; blocks forever
    pub fn run(mut self) {
        loop {
            // first process any commands
            while let Ok(command) = self.commands.try_recv() {
                match command {
                    ReplayManagerCommand::Record(f) => {
                        let mut replay_render = self.replay_render.write().unwrap();
                        if replay_render.recording {
                            return;
                        }
                        if replay_render.filename != f {
                            self.current_frame = 0;
                        }
                        self.frames.truncate(self.current_frame + 1);
                        replay_render.filename = f;
                        replay_render.recording = true;
                    }
                    ReplayManagerCommand::Playback(f) => {
                        let mut replay_render = self.replay_render.write().unwrap();
                        if !replay_render.recording {
                            return;
                        }
                        replay_render.recording = false;
                        replay_render.paused = true;
                        if replay_render.filename != f {
                            replay_render.filename = f;
                            self.current_frame = 0;
                        }
                    }

                    ReplayManagerCommand::RecordPhys(t) => {
                        if self.replay_render.read().unwrap().recording {
                            self.record_frame(RecordType::PhysRender, t, None)
                        }
                    }
                    ReplayManagerCommand::RecordPacman(t) => {
                        let filename = self.replay_render.read().unwrap().filename.to_owned();
                        if self.replay_render.read().unwrap().recording {
                            self.record_frame(RecordType::PacmanRender, t, Some(&filename))
                        }
                    }
                    ReplayManagerCommand::Beginning => {
                        self.current_frame = 0;
                    }
                    ReplayManagerCommand::StepBack => {
                        if self.current_frame != 0 {
                            self.current_frame -= 1;
                        }
                    }
                    ReplayManagerCommand::StepForward => {
                        if self.current_frame + 1 < self.frames.len() {
                            self.current_frame += 1;
                        }
                    }
                    ReplayManagerCommand::End => {
                        self.current_frame = self.frames.len() - 1;
                    }
                }
            }

            // then do playback if necessary
            if !self.replay_render.read().unwrap().recording
                && !self.replay_render.read().unwrap().paused
                && self.current_frame + 1 < self.frames.len()
            {
                // advance the frame
                self.current_frame += 1;
                // emit the new frame
                self.pacman_render.write().unwrap().pacman_state =
                    bincode::deserialize(&self.frames[self.current_frame].data).unwrap();
                // sleep until the next frame
                if self.current_frame + 1 < self.frames.len() {
                    let time_diff = self.frames[self.current_frame + 1]
                        .timestamp
                        .duration_since(self.frames[self.current_frame].timestamp)
                        .unwrap()
                        .as_secs_f32();
                    thread::sleep(std::time::Duration::from_secs_f32(
                        time_diff / self.replay_render.read().unwrap().playback_speed,
                    ));
                } else {
                    thread::sleep(std::time::Duration::from_secs_f32(0.1));
                }
            } else if !self.replay_render.read().unwrap().recording {
                let state: PacmanStateRenderInfo =
                    bincode::deserialize(&self.frames[self.current_frame].data).unwrap();
                let mut old_state = self.pacman_render.write().unwrap();
                old_state.pacman_state = state.pacman_state;
                old_state.agent_setup = state.agent_setup;
                thread::sleep(std::time::Duration::from_secs_f32(0.1));
            }
        }
    }

    /// Records one frame of generic data
    fn record_frame(
        &mut self,
        record_type: RecordType,
        timestamp: SystemTime,
        filename: Option<&String>,
    ) {
        let data;

        match record_type {
            RecordType::PhysRender => {
                let phys_render = self.phys_render.read().unwrap();
                data = bincode::serialize(&*phys_render).unwrap();
            }
            RecordType::PacmanRender => {
                let pacman_render = self.pacman_render.read().unwrap();
                data = bincode::serialize(&*pacman_render).unwrap();
            }
        }

        self.frames.push(ReplayFrame {
            record: record_type,
            data,
            timestamp,
        });

        self.current_frame += 1;

        if record_type == RecordType::PacmanRender {
            if let Some(_filename) = filename {
                // disable writing for now, it takes too long
                // self.write(filename);
            }
        }
    }

    /// Write into the file
    pub fn write(&self, filename: &String) {
        let mut file = std::fs::File::create(filename).unwrap();
        bincode::serialize_into(&mut file, &self.frames).unwrap();
    }
}
