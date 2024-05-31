use core::convert::Into;
use core::fmt::Display;

/// Type for holding temperature readings with context, in a fixed point manner
#[derive(PartialEq, Eq, PartialOrd, Copy, Clone, Default)]
// Valid representation range: -128-128C
// so 8 integer bits. 23 fractional bits
pub struct Degrees(pub i32);

impl Degrees {
    #[must_use]
    pub const fn from_int(val: i32) -> Self {
        Self(val << 12)
    }
}

// Conversion of ADC readings to degrees is specific to ADC config and circuit implementation, provide here a conversion that specifies our circuit
impl From<i64> for Degrees {
    /// Create a new `Degrees` from a 12-bit ADC read of a thermistor in a 10k voltage divider
    fn from(val: i64) -> Self {
        // R1 integer format
        // log2 of this is 14
        const R1: i64 = 10_000;

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

        // This now uses fixed point bullshit
        // val came out of a 12 bit ADC, it is F12 (number of fractional bits)

        // Scale by 12 to counteract the scaling of the divisor, though that puts us in a u64
        let divisor: i64 = (1 << 12) - val;
        let quotient: i64 = (val * R1) << 12;
        let r_fixed: i64 = quotient / divisor;

        // scaling in r_fixed is 12 fractional bits now.

        // From curve fit on R-T table this is the function for a 2-point cal on 25deg and 50deg
        // C = −4.2725*R+65.753 // R is in kilo-ohms, not ohms

        // fixed point versions of the above
        let slope_fixed: i64 = (-4 << 12) - 1116;
        let int_fixed: i64 = (65 << 12) + 3084;

        let mut temperature = (((slope_fixed * r_fixed) / 1000) >> 12) + int_fixed;

        if temperature > i32::MAX.into() {
            temperature = i32::MAX.into();
        } else if temperature < i32::MIN.into() {
            temperature = i32::MIN.into();
        }

        let temperature = unsafe { i32::try_from(temperature).unwrap_unchecked() };

        // F12 in is F12 out when the coefficients are scaled appropriately
        Self(temperature)
    }
}

impl TryFrom<Degrees> for u32 {
    type Error = &'static str;
    fn try_from(val: Degrees) -> Result<Self, Self::Error> {
        if val.0 < 0 {
            Ok(0)
        } else {
            Self::try_from(val.0).map_err(|_e| "infallible?")
        }
    }
}

#[allow(clippy::cast_possible_wrap)]
impl From<u32> for Degrees {
    fn from(value: u32) -> Self {
        if value > i32::MAX as u32 {
            Self(i32::MAX)
        } else {
            Self(value as i32)
        }
    }
}

impl Display for Degrees {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0 >> 12)
    }
}

// impl Format for Degrees {
//     fn format(&self, fmt: defmt::Formatter) {
//         defmt::write!(fmt, "{}", self.0);
//     }
// }
