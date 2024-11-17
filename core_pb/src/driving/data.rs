use crate::constants::ROBOT_LOGS_BUFFER;
use crate::driving::{EmbassyInstant, RobotBehavior};
use crate::messages::{
    ExtraImuData, ExtraOptsAtomicTypes, FrequentServerToRobot, NetworkStatus, RobotToServerMessage,
    SensorData,
};
use crate::names::RobotName;
use array_init::array_init;
use atomic::Atomic;
use core::sync::atomic::Ordering;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use portable_atomic::{AtomicBool, AtomicF32, AtomicI32, AtomicI8};

/// Each robot should have exactly one. Some fields are managed by core_pb, but (when noted)
/// implementations are responsible for updating values
pub struct RobotInterTaskData<const WHEELS: usize, Implementation: RobotBehavior> {
    /// Robot's name, to distinguish it from other robots, is provided on startup
    pub name: RobotName,
    /// An instant representing the time the shared struct was created
    pub created_at: EmbassyInstant,

    //
    // ------------------- INTER TASK DATA -------------------
    //
    /// Tasks may use this channel to queue messages to be sent back to the server
    ///
    /// If no active connection is available, the channel may fill up and remain full
    pub server_outgoing_queue: Channel<CriticalSectionRawMutex, RobotToServerMessage, 64>,

    /// Information gathered by the peripherals task will be posted here for network and motors
    pub sensors: Watch<CriticalSectionRawMutex, SensorData, 2>,
    /// The current network status, updated by network task
    pub network_status: Watch<CriticalSectionRawMutex, (NetworkStatus, Option<[u8; 4]>), 2>,
    /// Configuration from the server that may change frequently, updated by network task
    pub config: Watch<CriticalSectionRawMutex, FrequentServerToRobot, 2>,
    /// Utilization percentage for the three tasks
    pub utilization: [AtomicF32; 3],

    //
    // ------------------- ROBOT -> CORE DATA -------------------
    //
    /// Estimated motor speeds
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_motor_speeds: Signal<CriticalSectionRawMutex, [f32; WHEELS]>,
    /// An estimation of the absolute orientation of the robot
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_angle: Signal<CriticalSectionRawMutex, Result<f32, Implementation::PeripheralsError>>,
    /// Individual IMU sensor information
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_extra_imu_data: Signal<CriticalSectionRawMutex, ExtraImuData>,
    /// Readings from the distance sensors, in order of angle 0, 90, 180, 270
    ///
    /// It is the responsibility of the implementation to update this field.
    ///
    /// - Err(_) indicates that something is wrong with the sensor and the reading can't be trusted
    /// - Ok(None) indicates that the sensor is working, but didn't detect any object in its range
    /// - Ok(x) indicates an object x grid units in front of the sensor
    pub sig_distances:
        [Signal<CriticalSectionRawMutex, Result<Option<f32>, Implementation::PeripheralsError>>; 4],
    /// The battery level of the robot, in volts
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_battery: Signal<CriticalSectionRawMutex, Result<f32, Implementation::PeripheralsError>>,
    /// Logging bytes from defmt
    ///
    /// It is the responsibility of the implementation to update this field.
    pub defmt_logs: Pipe<CriticalSectionRawMutex, ROBOT_LOGS_BUFFER>,

    //
    // ------------------- EXTRA -------------------
    //
    extra_opts: ExtraOptsAtomicTypes,
    extra_indicators: ExtraOptsAtomicTypes,
}

fn make_extra_atomic_types() -> ExtraOptsAtomicTypes {
    (
        array_init(|_| AtomicBool::new(false)),
        array_init(|_| AtomicF32::new(0.0)),
        array_init(|_| AtomicI8::new(0)),
        array_init(|_| AtomicI32::new(0)),
    )
}

impl<const W: usize, PE: RobotBehavior> RobotInterTaskData<W, PE> {
    pub fn new(name: RobotName) -> Self {
        Self {
            name,
            created_at: EmbassyInstant::default(),

            server_outgoing_queue: Channel::new(),

            sensors: Watch::new(),
            network_status: Watch::new_with((NetworkStatus::NotConnected, None)),
            config: Watch::new(),
            utilization: array_init(|_| AtomicF32::new(0.0)),

            sig_motor_speeds: Default::default(),
            sig_angle: Default::default(),
            sig_extra_imu_data: Default::default(),
            sig_distances: Default::default(),
            sig_battery: Default::default(),
            defmt_logs: Pipe::new(),

            extra_opts: make_extra_atomic_types(),
            extra_indicators: make_extra_atomic_types(),
        }
    }

    #[deprecated = "Extra options should only be used for temporary testing"]
    pub fn get_extra_bool_opt(&self, index: usize) -> Option<bool> {
        self.extra_opts
            .0
            .get(index)
            .map(|b| b.load(Ordering::Relaxed))
    }

    #[deprecated = "Extra options should only be used for temporary testing"]
    pub fn get_extra_f32_opt(&self, index: usize) -> Option<f32> {
        self.extra_opts
            .1
            .get(index)
            .map(|b| b.load(Ordering::Relaxed))
    }

    #[deprecated = "Extra options should only be used for temporary testing"]
    pub fn get_extra_i8_opt(&self, index: usize) -> Option<i8> {
        self.extra_opts
            .2
            .get(index)
            .map(|b| b.load(Ordering::Relaxed))
    }

    #[deprecated = "Extra options should only be used for temporary testing"]
    pub fn get_extra_i32_opt(&self, index: usize) -> Option<i32> {
        self.extra_opts
            .3
            .get(index)
            .map(|b| b.load(Ordering::Relaxed))
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_bool_indicator(&self, index: usize, value: bool) {
        self.extra_indicators
            .0
            .get(index)
            .map(|b| b.store(value, Ordering::Relaxed));
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_f32_indicator(&self, index: usize, value: f32) {
        self.extra_indicators
            .1
            .get(index)
            .map(|b| b.store(value, Ordering::Relaxed));
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_i8_indicator(&self, index: usize, value: i8) {
        self.extra_indicators
            .2
            .get(index)
            .map(|b| b.store(value, Ordering::Relaxed));
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_i32_indicator(&self, index: usize, value: i32) {
        self.extra_indicators
            .3
            .get(index)
            .map(|b| b.store(value, Ordering::Relaxed));
    }
}
