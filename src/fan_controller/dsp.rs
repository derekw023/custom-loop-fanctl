use core::convert::TryInto;

/// Implements a 32 point moving average filter to reject some high frequency noise
pub struct MovingAverage<T> {
    buffer: [T; 32],
    index: usize,
    accumulator: u32,
}

impl MovingAverage<u16> {
    /// Create a moving average based filter to reject some high frequency noise
    pub const fn new() -> Self {
        Self {
            buffer: [0; 32],
            index: 0,
            accumulator: 0,
        }
    }

    /// Circular buffer with accumulator for moving average calculation
    // the cast is OK, because this is a 32 bit platform
    #[allow(clippy::cast_possible_truncation)]
    pub fn update(&mut self, val: u16) -> u16 {
        self.accumulator -= u32::from(self.buffer[self.index]);
        self.buffer[self.index] = val;
        self.accumulator += u32::from(self.buffer[self.index]);
        self.index += 1;
        self.index %= self.buffer.len();

        // Return new value
        (self.accumulator / (self.buffer.len() as u32))
            .try_into()
            .unwrap_or(0)
    }
}
