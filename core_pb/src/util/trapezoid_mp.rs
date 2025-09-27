
https://www.linearmotiontips.com/how-to-calculate-velocity/
https://www.fusybots.com/post/trajectoryplanningformanipulator-mathematicsbehindtrapezoidalplanner

// max acceleration
// start velocity
// end velocity
// max velocity


// return velocity

struct Trapezoid_MP {
    a: f32
    max_v: f32
    start_v: f32
    end_v: f32
}

impl Trapezoid_MP {
    // only calculating the first part
    
    fn get_velocity(&mut self, time: f32) -> f32 {

        // calculating velocity reached after time
        let vt = (self.a*time) + self.start_v;
        let v_effective_max = cmp::min(self.max_v, self.end_v);

        // calculating how long it takes to reach max_v
        let time_max = self.end_v - self.start_v / self.a;

        // continue increasing velocity
        cmp::min(vt, v_effective_max)
            
        
    }
}

