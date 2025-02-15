use crate::constants::ROBOT_LOGS_BUFFER;
use crate::driving::peripherals::RobotPeripheralsBehavior;
use crate::driving::RobotBehavior;
use crate::messages::{
    ExtraImuData, ExtraOptsAtomicTypes, ExtraOptsTypes, FrequentServerToRobot, MotorControlStatus,
    NetworkStatus, SensorData,
};
use crate::names::RobotName;
use crate::robot_definition::RobotDefinition;
use array_init::array_init;
use core::sync::atomic::Ordering;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use portable_atomic::{AtomicBool, AtomicF32, AtomicI32, AtomicI8, AtomicU64};

/// Each robot should have exactly one. Some fields are managed by core_pb, but (when noted)
/// implementations are responsible for updating values
#[allow(clippy::type_complexity)]
pub struct SharedRobotData<R: RobotBehavior + ?Sized> {
    /// Robot's name, to distinguish it from other robots, is provided on startup
    pub name: RobotName,
    /// The robot's physical characteristics
    pub robot_definition: RobotDefinition<3>,
    /// An instant representing the time the shared struct was created
    pub created_at: R::Instant,

    //
    // ----------- ENABLE/DISABLE DEVICES ----------
    // See core_pb/src/constants.rs for initial values
    pub enable_imu: AtomicBool,
    pub enable_extra_imu_data: AtomicBool,
    pub enable_dists: AtomicBool,
    pub enable_battery_monitor: AtomicBool,
    pub enable_display: AtomicBool,
    pub enable_gamepad: AtomicBool,
    pub display_loop_interval: AtomicU64,

    //
    // ------------------- INTER TASK DATA -------------------
    //
    /// Information gathered by the peripherals task will be posted here for network and motors
    pub sensors: Watch<CriticalSectionRawMutex, SensorData, 2>,
    /// The current network status, updated by network task
    pub network_status: Watch<CriticalSectionRawMutex, (NetworkStatus, Option<[u8; 4]>), 2>,
    /// Configuration from the server that may change frequently, updated by network task
    ///
    /// Note: some fields are loaded into other atomic primitives
    pub config: Watch<CriticalSectionRawMutex, FrequentServerToRobot, 2>,
    /// Motor control status, updated by motors task
    pub motor_control: Watch<CriticalSectionRawMutex, MotorControlStatus, 2>,
    /// Utilization percentage for the three tasks
    pub utilization: [AtomicF32; 3],

    //
    // ------------------- ROBOT -> CORE DATA -------------------
    //
    /// Estimated motor speeds
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_motor_speeds: Signal<CriticalSectionRawMutex, [f32; 3]>,
    /// An estimation of the absolute orientation of the robot
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_angle: Signal<
        CriticalSectionRawMutex,
        Result<f32, <R::Peripherals as RobotPeripheralsBehavior>::Error>,
    >,
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
    pub sig_distances: [Signal<
        CriticalSectionRawMutex,
        Result<Option<f32>, <R::Peripherals as RobotPeripheralsBehavior>::Error>,
    >; 4],
    /// The battery level of the robot, in volts
    ///
    /// It is the responsibility of the implementation to update this field.
    pub sig_battery: Signal<
        CriticalSectionRawMutex,
        Result<f32, <R::Peripherals as RobotPeripheralsBehavior>::Error>,
    >,
    pub buttons: [AtomicBool; 6],
    /// Logging bytes from defmt
    ///
    /// It is the responsibility of the implementation to update this field.
    pub defmt_logs: Pipe<CriticalSectionRawMutex, ROBOT_LOGS_BUFFER>,

    //
    // ------------------- EXTRA -------------------
    //
    pub extra_opts: ExtraOptsAtomicTypes,
    pub extra_indicators: ExtraOptsAtomicTypes,
}

fn make_extra_atomic_types() -> ExtraOptsAtomicTypes {
    (
        array_init(|_| AtomicBool::new(false)),
        array_init(|_| AtomicF32::new(0.0)),
        array_init(|_| AtomicI8::new(0)),
        array_init(|_| AtomicI32::new(0)),
    )
}

impl ExtraOptsTypes {
    pub fn store_into(&self, atomics: &ExtraOptsAtomicTypes) {
        for (i, x) in self.opts_bool.iter().enumerate() {
            atomics.0[i].store(*x, Ordering::Relaxed);
        }
        for (i, x) in self.opts_f32.iter().enumerate() {
            atomics.1[i].store(*x, Ordering::Relaxed);
        }
        for (i, x) in self.opts_i8.iter().enumerate() {
            atomics.2[i].store(*x, Ordering::Relaxed);
        }
        for (i, x) in self.opts_i32.iter().enumerate() {
            atomics.3[i].store(*x, Ordering::Relaxed);
        }
    }

    pub fn load_from(atomics: &ExtraOptsAtomicTypes) -> Self {
        let mut s = Self::default();
        for (i, x) in s.opts_bool.iter_mut().enumerate() {
            *x = atomics.0[i].load(Ordering::Relaxed);
        }
        for (i, x) in s.opts_f32.iter_mut().enumerate() {
            *x = atomics.1[i].load(Ordering::Relaxed);
        }
        for (i, x) in s.opts_i8.iter_mut().enumerate() {
            *x = atomics.2[i].load(Ordering::Relaxed);
        }
        for (i, x) in s.opts_i32.iter_mut().enumerate() {
            *x = atomics.3[i].load(Ordering::Relaxed);
        }
        s
    }
}

impl<R: RobotBehavior> SharedRobotData<R> {
    pub fn new(name: RobotName) -> Self {
        let config = FrequentServerToRobot::new(name);
        Self {
            name,
            robot_definition: RobotDefinition::new(name),
            created_at: R::Instant::default(),

            enable_imu: AtomicBool::new(config.enable_imu),
            enable_extra_imu_data: AtomicBool::new(config.enable_extra_imu_data),
            enable_dists: AtomicBool::new(config.enable_dists),
            enable_battery_monitor: AtomicBool::new(config.enable_battery_monitor),
            enable_display: AtomicBool::new(config.enable_display),
            enable_gamepad: AtomicBool::new(config.enable_gamepad),
            display_loop_interval: AtomicU64::new(config.display_loop_interval),

            sensors: Watch::new(),
            network_status: Watch::new_with((NetworkStatus::NotConnected, None)),
            config: Watch::new_with(config),
            motor_control: Watch::new(),
            utilization: array_init(|_| AtomicF32::new(0.0)),

            sig_motor_speeds: Default::default(),
            sig_angle: Default::default(),
            sig_extra_imu_data: Default::default(),
            sig_distances: Default::default(),
            sig_battery: Default::default(),
            buttons: array_init(|_| AtomicBool::new(false)),
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
        if let Some(b) = self.extra_indicators.0.get(index) {
            b.store(value, Ordering::Relaxed)
        }
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_f32_indicator(&self, index: usize, value: f32) {
        if let Some(b) = self.extra_indicators.1.get(index) {
            b.store(value, Ordering::Relaxed)
        }
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_i8_indicator(&self, index: usize, value: i8) {
        if let Some(b) = self.extra_indicators.2.get(index) {
            b.store(value, Ordering::Relaxed)
        }
    }

    #[deprecated = "Extra indicators should only be used for temporary testing"]
    pub fn set_extra_i32_indicator(&self, index: usize, value: i32) {
        if let Some(b) = self.extra_indicators.3.get(index) {
            b.store(value, Ordering::Relaxed)
        }
    }
}
