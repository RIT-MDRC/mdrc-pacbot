//! Keeps track of the average of a frequently changing value

/// Keeps track of the average of a frequently changing value
pub struct MovingAverage {
    /// The number of samples to keep in the average
    num_samples: usize,
    /// The samples that have been added to the average
    samples: Vec<f32>,
    /// The current index in the samples array
    index: usize,
    /// Whether this is the first time through the array
    first_samples: bool,
}

impl MovingAverage {
    /// Creates a new MovingAverage
    pub fn new(num_samples: usize) -> Self {
        Self {
            num_samples,
            samples: vec![0.0; num_samples],
            index: 0,
            first_samples: true,
        }
    }

    /// Adds a sample to the average
    pub fn add_sample(&mut self, sample: f32) {
        self.samples[self.index] = sample;
        self.index = (self.index + 1) % self.num_samples;

        if self.index == 0 {
            self.first_samples = false;
        }
    }

    /// Returns the average of all samples
    pub fn average(&self) -> f32 {
        if self.first_samples {
            if self.index == 0 {
                0.0
            } else {
                self.samples.iter().take(self.index).sum::<f32>() / self.index as f32
            }
        } else {
            self.samples.iter().sum::<f32>() / self.num_samples as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moving_average() {
        let mut average = MovingAverage::new(3);

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
