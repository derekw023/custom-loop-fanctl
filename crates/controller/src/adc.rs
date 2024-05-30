use pimoroni_tiny2040::hal::{
    adc::{Adc, AdcFifo, AdcPin},
    pac::{ADC, RESETS},
};

use crate::util::ThermistorPin;

pub struct Token<'a> {
    pub adc_fifo: AdcFifo<'a, u16>,
    sensor_pin: AdcPin<ThermistorPin>,
}

static mut STATIC_ADC: Option<Adc> = None;

impl Token<'_> {
    /// Consume ADC and provide constructed HAL structure (adc will be paused)
    pub fn new(adc: ADC, resets: &mut RESETS, sensor_pin: ThermistorPin) -> Option<Self> {
        // 1024 sps by USB clock trusting the documented factors
        let adc = Adc::new(adc, resets);
        let s_adc = unsafe {
            STATIC_ADC = Some(adc);
            STATIC_ADC.as_mut().unwrap_unchecked()
        };

        let mut sensor_pin = AdcPin::new(sensor_pin).unwrap();

        let fifo = s_adc
            .build_fifo()
            .set_channel(&mut sensor_pin)
            .clock_divider(46874, 0)
            .enable_dma()
            .start_paused();

        Some(Token {
            adc_fifo: fifo,
            sensor_pin,
        })
    }
}
