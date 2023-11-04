//! Keeps track of the average of a frequently changing value

/// Keeps track of the average of a frequently changing value
pub struct MovingAverage {
    /// The number of samples to keep in the average
    num_samples: usize,
    /// The samples that have been added to the average
    samples: Vec<f32>,
    /// The current index in the samples array
    index: usize,
}

impl MovingAverage {
    /// Creates a new MovingAverage
    pub fn new(num_samples: usize, default_average: f32) -> Self {
        Self {
            num_samples,
            samples: vec![default_average; num_samples],
            index: 0,
        }
    }

    /// Adds a sample to the average
    pub fn add_sample(&mut self, sample: f32) {
        self.samples[self.index] = sample;
        self.index = (self.index + 1) % self.num_samples;
    }

    /// Returns the average of all samples
    pub fn average(&self) -> f32 {
        self.samples.iter().sum::<f32>() / self.num_samples as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average() {
        let mut average = MovingAverage::new(3, 0.0);

        average.add_sample(1.0);
        average.add_sample(2.0);
        average.add_sample(3.0);
        assert_eq!(average.average(), 2.0);

        average.add_sample(4.0);
        assert_eq!(average.average(), 3.0);

        average.add_sample(5.0);
        assert_eq!(average.average(), 4.0);
    }
}
