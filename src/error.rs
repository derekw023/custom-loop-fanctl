use rp2040_hal::clocks::InitError;
pub enum Error {
    ClockError(InitError),
}

// impl defmt::Format for Error {
//     fn format(&self, fmt: defmt::Formatter) {
//         match self {
//             Error::ClockError(init_error) => match init_error {
//                 InitError::XoscErr(xe) => xe.format(fmt),
//                 InitError::PllError(pe) => pe.format(fmt),
//                 InitError::ClockError(ce) => match ce {
//                     rp2040_hal::clocks::ClockError::CantIncreaseFreq => {
//                         defmt::write!(fmt, "Can't Increse Frequency");
//                     }
//                     rp2040_hal::clocks::ClockError::FrequencyTooHigh => {
//                         defmt::write!(fmt, "Frequency Too High");
//                     }
//                     rp2040_hal::clocks::ClockError::FrequencyTooLow => {
//                         defmt::write!(fmt, "Frequency Too Low");
//                     }
//                     _ => defmt::write!(fmt, "Unknown clock error"),
//                 },
//             },
//         }
//     }
// }

impl From<InitError> for Error {
    fn from(value: InitError) -> Self {
        Self::ClockError(value)
    }
}
