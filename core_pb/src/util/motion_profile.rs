use std::cmp;

struct MotionProfile {
    a: f32,
    max_v: f32,
    current_v: f32,
    end_v: f32
}

impl MotionProfile {
    // only calculating the first part
    
    /**
     * Gets the next velocity in the motion profile given the time elapsed.
     * # Arguments
     * * `time` - The time to move in seconds
     */
    fn get_next_v(&mut self, time: f32) -> f32 {

        // calculating velocity reached after time
        let vt = (self.a*time) + self.current_v;
        let v_effective_max = cmp::min(self.max_v, self.end_v);

        // next velocity is min
        let new_v = cmp::min(vt, v_effective_max);
        self.current_v = new_v;
        new_v
    }
}

