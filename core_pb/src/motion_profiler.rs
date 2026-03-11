/*  Assume path is straight
    Params max velocity, position/distance, acceleration


    Consider using accel, maintain, decel enum

    2 phases:
        trajectory planning - how many phases (2 or 3), how long is each phase
        trajectory streaming - plug in values and calculate
*/ 

use nalgebra::Point2;

pub struct MpState {
    pub vel: f32,
    pub pos: f32
}

pub enum MpPhase {
    ACCEL,
    STATIC,
    DECEL
}

pub struct MotionProfiler {
    max_vel: f32,
    max_accel: f32,
    // 3 phase if true, 2 phase if false, consider changing for an enum. false by default
    trapezoidal: bool,
    phase: MpPhase,
    accel_endpoint: f32,
    static_endpoint: f32,
    decel_endpoint: f32,
    trajectory_ready: bool,
}

impl MotionProfiler {
    pub fn new (max_vel: f32, max_accel: f32) -> Self {
        return MotionProfiler {
            max_vel: max_vel, 
            max_accel: max_accel, 
            trapezoidal: false, 
            phase: MpPhase::STATIC,
            accel_endpoint: 0.0,
            static_endpoint: 0.0,
            decel_endpoint: 0.0,
            trajectory_ready: false
        }
    }

    /**
     * Decides the number of phases in the profile (2 or 3) and the length of each phase
     */
    pub fn plan_trajectory (
        &mut self,
        setpoint: MpState, 
        goal: MpState
    ) {
        // Determine type of profiling

        // total amount of dist in the path
        let dist = goal.pos - setpoint.pos;
        // how much dist is required to get to max velocity
        let dist_cap = calc_dist(&self.max_vel, &setpoint.vel, &self.max_accel, &dist);
        // println!("DIST_CAP: {}", dist_cap);

        if dist > dist_cap * 2.0 {
            self.trapezoidal = true;
        }

        // Determine length of phases

        // end of acceleration
        self.accel_endpoint = calc_time(&self.max_vel, &0.0, &self.max_accel) / (2.0 * self.max_accel);
        // end of static
        self.static_endpoint = if self.trapezoidal {
            dist - (2.0 * self.accel_endpoint)
        } else {
            0.0
        };
        // end of deceleration
        self.decel_endpoint = self.accel_endpoint + self.static_endpoint;

        self.trajectory_ready = true;
    }

    /**
     * Calculates the expected velocity for a given point in time
     */
    pub fn stream_trajectory (
        &mut self,
        setpoint: MpState,
        goal: MpState
    ) -> f32 {

        //    let mut v = 0.0;
        
        // // if time > t0 && time < t1 {
        // //     v = self.max_accel * (time - t0);
        // //     println!("V1: {}", v);
        // // } else if time > t1 && time < t2 {
        // //     v = self.max_accel * (t1 - t0);
        // //     println!("V2: {}", v);
        // // } else if time > t2 && time < t3 {
        // //     v = self.max_accel * (t3 - time);
        // //     println!("V3: {}", v);
        // // }
        // if v.abs() > self.max_vel {
        //     v = self.max_vel;
        //     println!("V4: {}", v);
        // }

        // return v.abs();
        return 0.0;
    }
}

fn calc_dist (
    vf: &f32,
    vi: &f32,
    a: &f32,
    d: &f32
) -> f32 {
    return (vf.powi(2) - vi.powi(2)) / (2.0 * a * d);
}

fn calc_time (
    vf: &f32,
    vi: &f32,
    a: &f32,
) -> f32 {
    return (vf - vi) / a;
}


        // let time = (setpoint.pos)/setpoint.vel;
        // println!("TIME: {}", time);
        // let t0 = 0.0; // should be 0
        // println!("T0: {}", t0);
        // let t1 = (self.max_vel - setpoint.vel) / self.max_accel; // t1 is the time when the robot reaches max velocity
        // println!("T1: {}", t1);
        // let t2 = (dist - (2.0 * dist_cap)) / self.max_vel; // t2 is the start time of deceleration
        // println!("T2: {}", t2);
        // let t3 = t2 + t1; // t3 is the end time of deceleration
        // println!("T3: {}", t3);