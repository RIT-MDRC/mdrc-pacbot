
https://www.linearmotiontips.com/how-to-calculate-velocity/
https://www.fusybots.com/post/trajectoryplanningformanipulator-mathematicsbehindtrapezoidalplanner

// max acceleration
// start velocity
// end velocity
// max velocity


// return velocity

struct Trapezoid_MP {
    max_a: f32
    max_v: f32
    start_v: f32
    end_v: f32
}

impl Trapezoid_MP {
    // only calculating the first part
    
    fn get_velocity(&mut self, time: f32) -> f32 {

        // we're not calculating the acceleration
        // Do we use tha max acceleration
        // does vc represent change in velocity? Why not the initial velocity
        let v_c = (self.max_a*time) + (self.end_v - self.start_v);
        if(v_c < self.max_v) {
            v_c
        }
        else {
            self.max_v
        }
            
        
    }
}

