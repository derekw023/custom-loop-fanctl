use super::temperature::{Degrees, TemperatureSensor};
use core::convert::TryInto;
use embedded_hal::adc::Channel;
use embedded_hal::adc::OneShot;
use embedded_hal::PwmPin;

/// Contains state and APIs to implement a simple proportional controller with saturation
pub struct FanController<'a, T, PIN, ADC, OUTPIN> {
    max_duty: T,
    min_duty: T,
    // Do not store max and min temp, as they are only necessary for calculating the curve params. cache those instead

    // Cached curve fit calculations
    m: f32,
    b: f32,

    /// Cached last temperature reading
    pub last_temp: Degrees,
    pub current_duty: T,

    /// Temperature Sensor
    sensor: TemperatureSensor<ADC, T, PIN>,
    out_pin: &'a mut OUTPIN,
}

impl<'a, ADC, PIN, OUTPIN> FanController<'a, u16, PIN, ADC, OUTPIN>
where
    ADC: OneShot<ADC, u16, PIN>,
    PIN: Channel<ADC, ID = u8>,
    OUTPIN: PwmPin,
    <OUTPIN as embedded_hal::PwmPin>::Duty: core::convert::From<u16>,
{
    /// Creates a new fan controller with the specified parameters
    ///
    /// # Arguments
    /// *  `max_duty` - The maximum duty cycle that shall be returned
    /// *  `min_duty` - The minimum duty cycle that shall be returned
    /// *  `max_temp` - High temperature saturation point, IE the temperature at which the returned duty will be `max_duty`
    /// *  `min_temp` - Low temperature saturation point
    pub fn new(
        pin: PIN,
        out_pin: &'a mut OUTPIN,
        max_duty: u16,
        min_duty: u16,
        max_temp: Degrees,
        min_temp: Degrees,
    ) -> Self {
        // Calculate the slope at construction instead of on update
        let diff: f32 = (max_duty - min_duty).try_into().unwrap_or(0.0);
        let min_duty_f: f32 = min_duty.try_into().unwrap_or(0.0);

        let slope: f32 = diff / (max_temp.0 - min_temp.0);
        let b = min_duty_f - slope * min_temp.0;

        let sensor = TemperatureSensor::new(pin);

        FanController {
            max_duty,
            min_duty,
            m: slope,
            b,
            current_duty: 0,
            last_temp: Degrees(65.),
            sensor,
            out_pin,
        }
    }

    /// Implement the configured fan curve and return a duty cycle
    pub fn fan_curve(&mut self, converter: &mut ADC) {
        // read temperature
        self.last_temp = self.sensor.read_temp(converter);

        // Simple linear fit with saturation, IE basic proportional-only PID loop
        let desired_duty = self.m * self.last_temp.0 + self.b;

        self.current_duty = if desired_duty > f32::from(self.max_duty) {
            self.max_duty
        } else if desired_duty < f32::from(self.min_duty) {
            self.min_duty
        } else {
            desired_duty as u16
        };

        self.out_pin.set_duty(self.current_duty.into());
    }
}
