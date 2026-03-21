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

pub struct MotionProfiler {
    max_vel: f32,
    max_accel: f32,
    // 3 phase if true, 2 phase if false, consider changing for an enum. false by default
    trapezoidal: bool,
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
        self.accel_endpoint = calc_time(&self.max_vel, &setpoint.vel, &self.max_accel);
        // end of static
        self.static_endpoint = if self.trapezoidal {
            (dist - (2.0 * self.accel_endpoint)) / self.max_vel
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
        setpoint: MpState
    ) -> f32 {

        if !self.trajectory_ready {
            panic!("Trajectory not planned");
        }

        let t = setpoint.pos / setpoint.vel;
        let t0 = 0.0;
        let t1 = self.accel_endpoint;
        let t2 = self.static_endpoint;
        let t3 = self.decel_endpoint;

        let mut v = 0.0;

        if t > t0 && t < t1 {
            v = self.max_accel * (t - t0);
        } else if t > t1 && t < t2 {
            v = self.max_accel * (t1 - t0);
        } else if t > t2 && t < t3 {
            v = -self.max_accel * (t3 - t);
        }

        if v.abs() > self.max_vel {
            v = self.max_vel;
        }
        
        return v.abs();
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