use std::error::Error;
use std::fmt::{Formatter, Result, Display};

#[derive(Debug)]
pub struct BootstrapError {
    pub details: String,
    pub code: u32,
}

impl BootstrapError {
    pub fn new(msg: &str) -> BootstrapError {
        BootstrapError {
            details: msg.to_string(),
            code: 0, //TODO error codes.
        }
    }
}

impl Display for BootstrapError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}, {}", self.details, self.code)
    }
}

impl Error for BootstrapError {
    fn description(&self) -> &str {
        &self.details
    }
}
