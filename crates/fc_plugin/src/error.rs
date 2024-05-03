use interoptopus::ffi_type;
use interoptopus::patterns::result::FFIError;

// Some Error used in your application.
#[derive(Debug)]
pub enum Error {
    Bad,
}

// The error FFI users should see
#[ffi_type(patterns(ffi_error))]
#[repr(C)]
#[derive(Debug)]
pub enum FanctlFFIError {
    Ok = 0,
    NullPassed = 1,
    Panic = 2,
    OtherError = 3,
}

// Gives special meaning to some of your error variants.
impl FFIError for FanctlFFIError {
    const SUCCESS: Self = Self::Ok;
    const NULL: Self = Self::NullPassed;
    const PANIC: Self = Self::Panic;
}

// How to map an `Error` to an `MyFFIError`.
impl From<Error> for FanctlFFIError {
    fn from(x: Error) -> Self {
        match x {
            Error::Bad => Self::OtherError,
        }
    }
}
