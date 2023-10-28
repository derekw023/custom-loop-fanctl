use super::temperature::Degrees;

/// Contains state and APIs to implement a simple proportional controller with saturation
pub struct FanCurve<T> {
    pub max_duty: T,
    min_duty: T,
    // Do not store max and min temp, as they are only necessary for calculating the curve params. cache those instead

    // Cached curve fit calculations
    m: i32,
    b: i32,
}

impl FanCurve<u16> {
    /// Creates a new fan controller with the specified parameters
    ///
    /// # Arguments
    /// *  `max_duty` - The maximum duty cycle that shall be returned
    /// *  `min_duty` - The minimum duty cycle that shall be returned
    /// *  `max_temp` - High temperature saturation point, IE the temperature at which the returned duty will be `max_duty`
    /// *  `min_temp` - Low temperature saturation point
    pub const fn new(max_duty: u16, min_duty: u16, max_temp: Degrees, min_temp: Degrees) -> Self {
        // Calculate the slope at construction instead of on update
        let diff = max_duty - min_duty;

        // F12 here
        let slope = ((diff as i64) << 24) / (max_temp.0 as i64 - min_temp.0 as i64);
        let b: i32 = ((min_duty as i32) << 12) - ((slope * min_temp.0 as i64) >> 12) as i32;

        // Fan curve now operates on F12 fixed point math
        FanCurve {
            max_duty,
            min_duty,
            m: slope as i32,
            b,
        }
    }

    /// Implement the configured fan curve and return a duty cycle
    // math has been validated theoretically not to exceed that cast
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn fan_curve(&mut self, temp: Degrees) -> u16 {
        // Simple linear fit with saturation, IE basic proportional-only PID loop
        let temp: i64 = (self.m as i64 * temp.0 as i64) >> 12;
        let desired_duty = (temp as i64 + self.b as i64) >> 12;

        // desired duty needs to be reduced to integer form
        let desired_duty = desired_duty as u16;

        if desired_duty > self.max_duty {
            self.max_duty
        } else if desired_duty < self.min_duty {
            self.min_duty
        } else {
            desired_duty
        }
    }
}
