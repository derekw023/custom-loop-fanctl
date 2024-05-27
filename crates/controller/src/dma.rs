use crate::bsp;
use bsp::hal::pac::{DMA, RESETS};
use rp2040_hal::dma::{Channel, DMAExt, DynChannels};

pub(crate) struct Token {
    pub channels: DynChannels,
}

macro_rules! impl_ch_take {
    ($implname:ident, $dmach:ident, $chbignum:tt) => {
        pub(crate) fn $implname(&mut self) -> Option<Channel<$crate::bsp::hal::dma::$chbignum>> {
            self.channels.$dmach.take()
        }
    };
}

impl Token {
    pub(crate) fn new(dma: DMA, resets: &mut RESETS) -> Self {
        let channels = dma.dyn_split(resets);

        Self { channels }
    }

    impl_ch_take!(take_ch0, ch0, CH0);
    impl_ch_take!(take_ch1, ch1, CH1);
    impl_ch_take!(take_ch2, ch2, CH2);
    impl_ch_take!(take_ch3, ch3, CH3);
    impl_ch_take!(take_ch4, ch4, CH4);
    impl_ch_take!(take_ch5, ch5, CH5);
    impl_ch_take!(take_ch6, ch6, CH6);
    impl_ch_take!(take_ch7, ch7, CH7);
    impl_ch_take!(take_ch8, ch8, CH8);
    impl_ch_take!(take_ch9, ch9, CH9);
    impl_ch_take!(take_ch10, ch10, CH10);
    impl_ch_take!(take_ch11, ch11, CH11);
}
