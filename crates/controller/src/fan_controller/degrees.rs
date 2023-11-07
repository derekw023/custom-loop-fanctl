use core::fmt::Display;

/// Type for holding temperature readings with context, in a fixed point manner
#[derive(PartialEq, PartialOrd, Copy, Clone, Default)]
// Valid representation range: -128-128C
// so 8 integer bits. 23 fractional bits
pub struct Degrees(pub i32);

impl Degrees {
    pub const fn from_int(val: i32) -> Self {
        Self(val << 12)
    }
}

// Conversion of ADC readings to degrees is specific to ADC config and circuit implementation, provide here a conversion that specifies our circuit
impl From<u16> for Degrees {
    /// Create a new `Degrees` from a 12-bit ADC read of a thermistor in a 10k voltage divider
    #[allow(clippy::cast_lossless, clippy::cast_precision_loss)]
    fn from(val: u16) -> Self {
        // R1 integer format
        // log2 of this is 14
        const R1: u32 = 10_000;

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
        let r_fixed: i64 = ((val as i64 * R1 as i64) << 12) / ((1 << 12) - val as i64);

        // scaling in r_fixed is 12 fractional bits now.

        // From curve fit on R-T table this is the function for a 2-point cal on 25deg and 50deg
        // C = −4.2725*R+65.753 // R is in kilo-ohms, not ohms

        // fixed point versions of the above
        let slope_fixed: i64 = (-4 << 12) - 1116;
        let int_fixed: i64 = (65 << 12) + 3084;

        // F12 in is F12 out when the coefficients are scaled appropriately
        Degrees(
            i32::try_from((((slope_fixed * r_fixed) / 1000) >> 12) + int_fixed)
                .expect("Line mapping error"),
        )
    }
}

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
impl From<Degrees> for u32 {
    fn from(val: Degrees) -> u32 {
        if val.0 < 0 {
            0
        } else {
            val.0 as u32
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
