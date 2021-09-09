use core::convert::TryInto;

/// Implements a 32 point moving average filter to reject some high frequency noise
pub struct MovingAverage<T> {
    buffer: [T; 32],
    index: usize,
    accumulator: u32,
    /// read `current_output` this to get the latest output data
    pub current_output: T,
}

impl MovingAverage<u16> {
    /// Create a moving average based filter to reject some high frequency noise
    pub fn new() -> Self {
        Self {
            buffer: [0; 32],
            index: 0,
            accumulator: 0,
            current_output: 0,
        }
    }

    /// Circular buffer with accumulator for moving average calculation
    pub fn update(&mut self, val: u16) -> u16 {
        self.accumulator -= u32::from(self.buffer[self.index]);
        self.buffer[self.index] = val;
        self.accumulator += u32::from(self.buffer[self.index]);
        self.index += 1;
        self.index %= self.buffer.len();

        self.current_output = (self.accumulator / (self.buffer.len() as u32))
            .try_into()
            .unwrap_or(0);

        // Return new value
        self.current_output
    }
}
