#![allow(
    clippy::ignored_unit_patterns,
    clippy::useless_conversion,
    clippy::missing_errors_doc
)]
use interoptopus::{ffi_service, ffi_service_ctor, ffi_service_method, ffi_type};
mod error;

use crate::error::{Error, FanctlFFIError};

#[ffi_type(opaque)]
pub struct SimpleService {
    pub some_value: u32,
}

#[ffi_service(error = "FanctlFFIError", prefix = "simple_service_")]
impl SimpleService {
    #[ffi_service_ctor]
    pub fn new_with(some_value: u32) -> Result<Self, Error> {
        Ok(Self { some_value })
    }

    pub fn maybe_fails(&self, x: u32) -> Result<u32, Error> {
        Ok(x)
    }

    #[ffi_service_method(on_panic = "return_default")]
    #[must_use]
    pub fn just_return_value(&self) -> u32 {
        self.some_value
    }
}
