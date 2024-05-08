use rp2040_hal::{adc::AdcFifo, Adc};

use crate::util::ControllerPeripherals;

static ADC_FIFO: Option<AdcFifo<u16>> = None;

pub struct AdcToken {
    placeholder_sample: u16,
}

/// Consume ADC and provide constructed HAL structure
pub fn configure_adc(peripherals: &mut ControllerPeripherals) -> Option<AdcToken> {
    let converter = peripherals.adc.take()?;
    let fifo = Adc::new(converter, &mut peripherals.resets)
        .build_fifo()
        .enable_dma()
        .start_paused();

    unsafe { ADC_FIFO = Some(fifo) }

    Some(AdcToken {
        placeholder_sample: 0,
    })
}

pub fn start_dma() {
    unimplemented!()
}

fn dma_irq() {
    unimplemented!()
}
