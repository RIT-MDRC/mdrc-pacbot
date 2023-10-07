//! Records and replays GUI data

use crate::gui::{PacmanStateRenderInfo, PhysicsRenderInfo};
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
    /// Set the playback speed - 1.0 is normal speed
    Speed(f32),

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
    /// recording, vs playback
    recording: bool,
    /// File to record to or playback from
    filename: String,
    /// Whether playback is paused
    paused: bool,
    /// Playback speed; 1.0 is normal speed
    playback_speed: f32,
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

        commands: Receiver<ReplayManagerCommand>,
    ) -> ReplayManager {
        // create default timestamped filename with human readable date
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("replays/replay-{}.bin", time);

        let frames = vec![];

        let mut s = Self {
            recording: true,
            filename,
            paused: true,
            playback_speed: 0.0,

            commands,

            phys_render,
            pacman_render,

            frames,
            current_frame: 0,
        };

        // initial state
        s.record_frame(RecordType::PhysRender, SystemTime::now());
        s.record_frame(RecordType::PacmanRender, SystemTime::now());

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
                        if self.recording {
                            return;
                        }
                        self.filename = f;
                        self.recording = true;
                        // TODO
                    }
                    ReplayManagerCommand::Playback(f) => {
                        if !self.recording {
                            return;
                        }
                        self.filename = f;
                        self.recording = false;
                        self.paused = true;
                        // TODO
                    }
                    ReplayManagerCommand::Speed(s) => self.playback_speed = s,

                    ReplayManagerCommand::RecordPhys(t) => {
                        self.record_frame(RecordType::PhysRender, t)
                    }
                    ReplayManagerCommand::RecordPacman(t) => {
                        self.record_frame(RecordType::PacmanRender, t)
                    }
                }
            }

            // then do playback if necessary
            if !self.recording && !self.paused && self.current_frame + 1 < self.frames.len() {
                // TODO
                // advance the frame
                self.current_frame += 1;
                // emit the new frame
                // sleep until the next frame
                let time_diff = self.frames[self.current_frame]
                    .timestamp
                    .duration_since(self.frames[self.current_frame - 1].timestamp)
                    .unwrap()
                    .as_secs_f32();
                thread::sleep(std::time::Duration::from_secs_f32(
                    time_diff / self.playback_speed,
                ));
            }
        }
    }

    /// Records one frame of generic data
    fn record_frame(&mut self, record_type: RecordType, timestamp: SystemTime) {
        if !self.recording {
            return;
        }

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
            self.write();
        }
    }

    /// Write into the file
    pub fn write(&self) {
        let mut file = std::fs::File::create(&self.filename).unwrap();
        bincode::serialize_into(&mut file, &self.frames).unwrap();
    }
}
