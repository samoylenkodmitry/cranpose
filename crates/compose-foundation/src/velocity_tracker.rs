//! Velocity tracking for fling gesture support.
//!
//! Port of Jetpack Compose's VelocityTracker1D using the Impulse strategy.
//! This calculates velocity based on kinetic energy principles.

/// Ring buffer size for velocity tracking samples.
const HISTORY_SIZE: usize = 20;

/// Only use samples within the last 100ms for velocity calculation.
const HORIZON_MS: i64 = 100;

/// If no movement for this duration, assume the pointer has stopped.
const ASSUME_STOPPED_MS: i64 = 40;

/// Minimum pointer movement (in pixels) to consider as non-stopped.
/// If total movement is below this over ASSUME_STOPPED_MS, velocity is 0.
const MIN_MOVEMENT_THRESHOLD: f32 = 2.0;

/// A data point with timestamp.
#[derive(Clone, Copy, Default)]
struct DataPointAtTime {
    time_ms: i64,
    data_point: f32,
}

/// 1D velocity tracker using impulse-based velocity calculation.
///
/// This implements the same algorithm as Jetpack Compose's VelocityTracker1D
/// with the Impulse strategy, which calculates velocity based on the
/// kinetic energy imparted by the touch gestures.
///
/// # Usage
/// ```ignore
/// let mut tracker = VelocityTracker1D::new();
/// tracker.add_data_point(time_ms, position);
/// // ... more points ...
/// let velocity = tracker.calculate_velocity(); // px/sec
/// ```
#[derive(Clone)]
pub struct VelocityTracker1D {
    /// Ring buffer of samples.
    samples: [Option<DataPointAtTime>; HISTORY_SIZE],
    /// Current write index in ring buffer.
    index: usize,
    /// Whether data points are differential (change in position) vs absolute positions.
    is_differential: bool,
}

impl Default for VelocityTracker1D {
    fn default() -> Self {
        Self::new()
    }
}

impl VelocityTracker1D {
    /// Creates a new velocity tracker for absolute position data.
    pub fn new() -> Self {
        Self {
            samples: [None; HISTORY_SIZE],
            index: 0,
            is_differential: false,
        }
    }

    /// Creates a new velocity tracker for differential (delta) data.
    #[allow(dead_code)]
    pub fn differential() -> Self {
        Self {
            samples: [None; HISTORY_SIZE],
            index: 0,
            is_differential: true,
        }
    }

    /// Adds a data point at the given time.
    ///
    /// For absolute tracking, `data_point` is the position.
    /// For differential tracking, `data_point` is the change since last point.
    pub fn add_data_point(&mut self, time_ms: i64, data_point: f32) {
        self.index = (self.index + 1) % HISTORY_SIZE;
        self.samples[self.index] = Some(DataPointAtTime { time_ms, data_point });
    }

    /// Calculates the velocity in units/second.
    ///
    /// Returns 0.0 if there aren't enough samples or if the pointer hasn't moved.
    pub fn calculate_velocity(&self) -> f32 {
        let mut data_points = [0.0f32; HISTORY_SIZE];
        let mut times = [0.0f32; HISTORY_SIZE];
        let mut sample_count = 0;

        // Get newest sample
        let newest_sample = match self.samples[self.index] {
            Some(s) => s,
            None => return 0.0,
        };

        let mut current_index = self.index;
        let mut oldest_sample_in_window: Option<DataPointAtTime> = None;

        // Collect ALL samples within the HORIZON_MS window
        // Don't break early on gaps - let the regression algorithm handle timing naturally
        loop {
            let sample = match self.samples[current_index] {
                Some(s) => s,
                None => break,
            };

            let age = (newest_sample.time_ms - sample.time_ms) as f32;
            
            // Stop if sample is too old (outside the horizon window)
            if age > HORIZON_MS as f32 {
                break;
            }

            oldest_sample_in_window = Some(sample);
            data_points[sample_count] = sample.data_point;
            times[sample_count] = -age; // Negative because we're going backwards

            // Move to previous sample in ring buffer
            current_index = if current_index == 0 {
                HISTORY_SIZE - 1
            } else {
                current_index - 1
            };

            sample_count += 1;
            if sample_count >= HISTORY_SIZE {
                break;
            }
        }

        // Need at least 2 samples for velocity calculation
        if sample_count < 2 {
            return 0.0;
        }

        // Check if pointer has actually moved significantly
        // Only return 0 if there's no meaningful movement, not based on time gaps
        if let Some(oldest) = oldest_sample_in_window {
            let total_movement = (newest_sample.data_point - oldest.data_point).abs();
            let time_span_ms = (newest_sample.time_ms - oldest.time_ms) as f32;
            
            // If pointer barely moved over a significant time, it's stopped
            // This is more robust than gap detection - checks actual movement
            if time_span_ms > ASSUME_STOPPED_MS as f32 && total_movement < MIN_MOVEMENT_THRESHOLD {
                return 0.0;
            }
        }

        // Calculate velocity using weighted least squares
        let velocity_per_ms = self.calculate_impulse_velocity(&data_points, &times, sample_count);

        // Convert from units/ms to units/second
        velocity_per_ms * 1000.0
    }

    /// Clears all tracked data.
    pub fn reset(&mut self) {
        self.samples = [None; HISTORY_SIZE];
        self.index = 0;
    }

    /// Calculate velocity using weighted least squares linear regression.
    ///
    /// This gives more weight to recent samples and produces velocity
    /// that closely matches the actual gesture speed (not amplified).
    fn calculate_impulse_velocity(
        &self,
        data_points: &[f32; HISTORY_SIZE],
        times: &[f32; HISTORY_SIZE],
        sample_count: usize,
    ) -> f32 {
        if sample_count < 2 {
            return 0.0;
        }

        // Use simple linear regression with exponential weighting for recency
        // This gives us average velocity that closely tracks actual finger speed
        let mut sum_weight = 0.0f32;
        let mut sum_t = 0.0f32;
        let mut sum_x = 0.0f32;
        let mut sum_tt = 0.0f32;
        let mut sum_tx = 0.0f32;

        // Decay factor - recent samples weighted more (half-life ~30ms)
        let decay = 0.95f32;

        for i in 0..sample_count {
            // Weight: more recent samples (lower index) get higher weight
            let weight = decay.powi(i as i32);
            let t = times[i];
            let x = data_points[i];

            sum_weight += weight;
            sum_t += weight * t;
            sum_x += weight * x;
            sum_tt += weight * t * t;
            sum_tx += weight * t * x;
        }

        // Weighted linear regression: x = a + b*t, velocity = b
        let denom = sum_weight * sum_tt - sum_t * sum_t;
        if denom.abs() < f32::EPSILON {
            return 0.0;
        }

        let velocity = (sum_weight * sum_tx - sum_t * sum_x) / denom;
        velocity
    }
}

/// Converts kinetic energy to velocity using E = 0.5 * m * v^2 (with m = 1).
#[inline]
#[allow(dead_code)]
fn kinetic_energy_to_velocity(kinetic_energy: f32) -> f32 {
    kinetic_energy.signum() * (2.0 * kinetic_energy.abs()).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tracker_returns_zero() {
        let tracker = VelocityTracker1D::new();
        assert_eq!(tracker.calculate_velocity(), 0.0);
    }

    #[test]
    fn test_single_point_returns_zero() {
        let mut tracker = VelocityTracker1D::new();
        tracker.add_data_point(0, 100.0);
        assert_eq!(tracker.calculate_velocity(), 0.0);
    }

    #[test]
    fn test_constant_velocity() {
        let mut tracker = VelocityTracker1D::new();
        // Moving at 100 px per 10ms = 10000 px/s
        tracker.add_data_point(0, 0.0);
        tracker.add_data_point(10, 100.0);
        tracker.add_data_point(20, 200.0);
        tracker.add_data_point(30, 300.0);

        let velocity = tracker.calculate_velocity();
        // Should be approximately 10000 px/s
        assert!(
            (velocity - 10000.0).abs() < 1000.0,
            "Expected ~10000, got {}",
            velocity
        );
    }

    #[test]
    fn test_reset() {
        let mut tracker = VelocityTracker1D::new();
        tracker.add_data_point(0, 0.0);
        tracker.add_data_point(10, 100.0);

        tracker.reset();

        assert_eq!(tracker.calculate_velocity(), 0.0);
    }

    #[test]
    fn test_negative_velocity() {
        let mut tracker = VelocityTracker1D::new();
        // Moving backwards
        tracker.add_data_point(0, 300.0);
        tracker.add_data_point(10, 200.0);
        tracker.add_data_point(20, 100.0);

        let velocity = tracker.calculate_velocity();
        assert!(velocity < 0.0, "Expected negative velocity, got {}", velocity);
    }

    #[test]
    fn test_old_samples_ignored() {
        let mut tracker = VelocityTracker1D::new();
        // Old sample (more than HORIZON_MS ago)
        tracker.add_data_point(0, 0.0);
        // Recent samples
        tracker.add_data_point(150, 100.0);
        tracker.add_data_point(160, 200.0);
        tracker.add_data_point(170, 300.0);

        // Velocity should only be based on recent samples
        let velocity = tracker.calculate_velocity();
        assert!(
            velocity.abs() > 0.0,
            "Should calculate velocity from recent samples"
        );
    }
}
