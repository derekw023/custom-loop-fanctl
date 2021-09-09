use super::dsp::MovingAverage;
use embedded_hal::adc::Channel;
use embedded_hal::adc::OneShot;

/// Type for holding temperature readings with context
#[derive(PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

// Conversion of ADC readings to degrees is specific to ADC config and circuit implementation, provide here a conversion that specifies our circuit
impl From<u16> for Degrees {
    /// Create a new `Degrees` from a 12-bit ADC read of a thermistor in a 10k voltage divider
    fn from(val: u16) -> Self {
        const R1: f32 = 10_000.;
        // Convert value to a floating point fraction of vdd, assuming 12 bit ADC
        let val_volts = val as f32 / (1 << 12) as f32;

        // IR = V, construct voltage divider equation
        // (VDD / (R1 + Rt)) * Rt = Vin = val_volts * VDD

        // val is in terms of VDD already, simplify
        // (Rt / (R1 + Rt)) = val_volts

        // Rearrange to measure Rt, given R1 is known
        // Rt = val_volts * (R1 + Rt)
        // Rt - val_volts * Rt = val_volts * R1
        // Rt * (1 - val_volts) = val_volts * R1)
        // Rt = val_volts * R1 / (1 - val_volts)
        // Rt = R1 / (1/val_volts - 1) // simplification to prove equivalence with previous version of this math
        let r = val_volts * R1 / (1. - val_volts);

        // From curve fit on R-T table this is the function for a 2-point cal on 25deg and 50deg
        // C = −4.2725*R+65.753 // R is in kilo-ohms, not ohms

        Degrees(-4.2725 * r / 1000. + 65.753)
    }
}

pub struct TemperatureSensor<ADC, Word, Pin> {
    sensor_pin: Pin,
    filter: MovingAverage<Word>,
    _unused: core::marker::PhantomData<ADC>,
}

impl<ADC, T> TemperatureSensor<ADC, u16, T>
where
    ADC: OneShot<ADC, u16, T>,
    T: Channel<ADC, ID = u8>,
{
    pub fn new(sensor_pin: T) -> Self {
        TemperatureSensor {
            sensor_pin,
            filter: MovingAverage::new(),
            _unused: core::marker::PhantomData,
        }
    }

    pub fn read_temp(&mut self, converter: &mut ADC) -> Degrees {
        match converter.read(&mut self.sensor_pin) {
            Ok(t) => self.filter.update(t).into(),
            Err(_) => 0u16.into(),
        }
    }
}
