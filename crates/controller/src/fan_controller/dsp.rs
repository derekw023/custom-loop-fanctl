use core::default::Default;

/// Implements a 32 point moving average filter to reject some high frequency noise
pub struct MovingAverage<T>
where
    T: Copy + Clone + Into<u32> + From<u32> + Default,
{
    buffer: [T; 32],
    index: usize,
    accumulator: u32,
}

impl<T> MovingAverage<T>
where
    T: Copy + Clone + Into<u32> + From<u32> + Default,
{
    /// Create a moving average based filter to reject some high frequency noise
    pub fn new() -> Self {
        Self {
            buffer: [Default::default(); 32],
            index: 0,
            accumulator: 0,
        }
    }

    /// Circular buffer with accumulator for moving average calculation
    // the cast is OK, because this is a 32 bit platform
    #[allow(clippy::cast_possible_truncation)]
    pub fn update(&mut self, val: T) -> T {
        self.accumulator -= self.buffer[self.index].into();
        self.buffer[self.index] = val;
        self.accumulator += self.buffer[self.index].into();
        self.index += 1;
        self.index %= self.buffer.len();

        // Return new value
        (self.accumulator / (self.buffer.len() as u32)).into()
    }
}
