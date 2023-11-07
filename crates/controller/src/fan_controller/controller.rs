use super::Degrees;

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
    pub fn new(max_duty: u16, min_duty: u16, max_temp: Degrees, min_temp: Degrees) -> Self {
        // Calculate the slope at construction instead of on update
        let diff = max_duty - min_duty;

        // F12 here
        let slope = (i64::from(diff) << 24) / (i64::from(max_temp.0) - i64::from(min_temp.0));
        let int: i64 = (i64::from(min_duty) << 12) - ((slope * i64::from(min_temp.0)) >> 12);

        // Fan curve now operates on F12 fixed point math
        FanCurve {
            max_duty,
            min_duty,
            m: i32::try_from(slope).expect("slope calculation error"),
            b: i32::try_from(int).expect("y-int calculation error"),
        }
    }

    /// Implement the configured fan curve and return a duty cycle
    pub fn fan_curve(&mut self, temp: Degrees) -> u16 {
        // Simple linear fit with saturation, IE basic proportional-only PID loop
        let temp: i64 = (i64::from(self.m) * i64::from(temp.0)) >> 12;
        let desired_duty = (temp + i64::from(self.b)) >> 12;

        if desired_duty > i64::from(self.max_duty) {
            self.max_duty
        } else if desired_duty < i64::from(self.min_duty) {
            self.min_duty
        } else {
            u16::try_from(desired_duty).expect("Duty cycle range error")
        }
    }
}
