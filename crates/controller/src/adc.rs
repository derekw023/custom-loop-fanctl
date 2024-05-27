use rp2040_hal::{
    adc::{Adc, AdcFifo},
    pac::{ADC, RESETS},
};

pub struct Token<'a> {
    adc: Adc,
    adc_fifo: AdcFifo<'a, u16>,
}

impl Token<'_> {
    /// Consume ADC and provide constructed HAL structure (adc will be paused)
    pub fn new(adc: ADC, resets: &mut RESETS) -> Option<Self> {
        // 1024 sps by USB clock trusting the documented factors
        let mut adc = Adc::new(adc, resets);
        let fifo = adc
            .build_fifo()
            .clock_divider(46874, 0)
            .enable_dma()
            .start_paused();

        Some(Token {
            adc,
            adc_fifo: fifo,
        })
    }
}
