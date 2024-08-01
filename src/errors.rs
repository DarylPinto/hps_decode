use thiserror::Error;
use winnow::error::{ContextError, ErrMode};

use crate::hps::COEFFICIENT_PAIRS_PER_CHANNEL;

#[derive(Error, Debug)]
pub enum HpsParseError {
    /// The first 8 bytes in the file are not ` HALPST\0`
    #[error("Invalid magic number. Expected ' HALPST\0'")]
    InvalidMagicNumber,

    /// The number of audio channels in the provided file is not supported by the library
    #[error("Only stereo is supported, but the provided file has {0} audio channel(s)")]
    UnsupportedChannelCount(u32),

    #[error("There was not enough data, {0:?} more bytes were needed")]
    Incomplete(winnow::error::Needed),

    #[error("Tried to parse, but encountered invalid data. Cause: {}",
    match .0.cause() {
        Some(cause) => cause.to_string(),
        None => "None".to_string()
    })]
    InvalidData(ContextError),
}

impl From<ErrMode<ContextError>> for HpsParseError {
    fn from(error: ErrMode<ContextError>) -> Self {
        match error {
            winnow::error::ErrMode::Incomplete(needed) => HpsParseError::Incomplete(needed),
            winnow::error::ErrMode::Backtrack(e) | winnow::error::ErrMode::Cut(e) => {
                HpsParseError::InvalidData(e)
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum HpsDecodeError {
    #[error("One of the audio frame headers contains a coefficient index of {0} which is invalid. Length of the coefficients array is {COEFFICIENT_PAIRS_PER_CHANNEL}")]
    InvalidCoefficientIndex(usize),
}
