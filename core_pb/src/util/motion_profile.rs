use std::cmp;

struct MotionProfile {
    a: f32
    max_v: f32
    start_v: f32
    end_v: f32
}

impl MotionProfile {
    // only calculating the first part
    
    fn get_next_v(&mut self, time: f32) -> f32 {

        // calculating velocity reached after time
        let vt = (self.a*time) + self.start_v;
        let v_effective_max = cmp::min(self.max_v, self.end_v);

        // next velocity is min
        cmp::min(vt, v_effective_max)
    }
}

