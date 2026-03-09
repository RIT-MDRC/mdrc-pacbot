// Assume path is straight
// Params max velocity, position/distance, acceleration

use nalgebra::Point2;

pub struct MpState {
    pub vel: f32,
    pub pos: f32
}

pub struct MotionProfiler {
    max_vel: f32,
    max_accel: f32
}

impl MotionProfiler {
    pub fn new (max_vel: f32, max_accel: f32) -> Self {
        return MotionProfiler {max_vel: max_vel, max_accel: max_accel}
    }

    pub fn calculate (
        &self,
        setpoint: MpState, 
        goal: MpState
    ) -> f32 {

        let dist = goal.pos - setpoint.pos;

        let dist_cap = vel_to_dist(&self.max_vel, &setpoint.vel, &self.max_accel, &dist);
        println!("DIST_CAP: {}", dist_cap);

        let time = (setpoint.pos)/setpoint.vel;
        println!("TIME: {}", time);
        let t0 = solve_time(&setpoint.vel, &self.max_accel, &dist);
        println!("T0: {}", t0);
        let t1 = (self.max_vel - setpoint.vel) / self.max_accel; // t1 is the time when the robot reaches max velocity
        println!("T1: {}", t1);
        let t2 = (dist - 2.0 * dist_cap) / self.max_vel; // t2 is the start time of deceleration
        println!("T2: {}", t2);
        let t3 = t2 + t1; // t3 is the end time of deceleration
        println!("T3: {}", t3);
        let mut v = 0.0;
        
        if time > t0 && time < t1 {
            v = self.max_accel * (time - t0);
            println!("V1: {}", v);
        } else if time > t1 && time < t2 {
            v = self.max_accel * (t1 - t0);
            println!("V2: {}", v);
        } else if time > t2 && time < t3 {
            v = self.max_accel * (t3 - time);
            println!("V3: {}", v);
        }
        if v.abs() > self.max_vel {
            v = self.max_vel;
            println!("V4: {}", v);
        }

        return v.abs();
    }
}

fn vel_to_dist (
    vf: &f32,
    vi: &f32,
    a: &f32,
    d: &f32
) -> f32 {
    return (vf.powi(2) - vi.powi(2)) / (2.0 * a * d);
}

fn solve_time (
    vi: &f32,
    a: &f32,
    s0: &f32
) -> f32 {
    // println!("vi: {}, a: {}, s0: {}", vi, a, s0);
    let time_a = (-vi - (vi.powi(2) - 4.0 * a * s0).abs().sqrt()) / a;
    let time_b = (-vi + (vi.powi(2) - 4.0 * a * s0).abs().sqrt()) / a;
    return if time_a > time_b { // have it return the greater one for testing but it might be wrong
        time_a
    } else {
        time_b
    };
}

pub fn calc_straight_away_speed (
    max_speed: f32,
    postion: Option<Point2<i8>>,
) -> f32 {
    let goal = MpState {vel: 4.0, pos: 5.0};
    let setpoint = MpState {vel: 0.0, pos: 1.0};

    let tmp = MotionProfiler {max_vel: 5.0, max_accel: 2.0};

    return tmp.calculate(setpoint, goal);
}