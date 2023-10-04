use nom::{error::ErrorKind, Needed};
use thiserror::Error;

use crate::hps::COEFFICIENT_PAIRS_PER_CHANNEL;

pub(crate) type NomByteInputError<'a> = nom::Err<nom::error::Error<&'a [u8]>>;

#[derive(Error, Debug)]
pub enum HpsParseError {
    /// The first 8 bytes in the file are not ` HALPST\0`
    #[error("Invalid magic number. Expected ' HALPST\0'")]
    InvalidMagicNumber,

    #[error("The file has zero audio channels")]
    ZeroAudioChannels,

    #[error("There was not enough data, {0:?} more bytes were needed")]
    Incomplete(Needed),

    #[error("Tried to parse with {0:?}, but encountered invalid data ({} bytes remaining)", .1.len())]
    InvalidData(ErrorKind, Vec<u8>),
}

impl From<NomByteInputError<'_>> for HpsParseError {
    fn from(error: NomByteInputError<'_>) -> Self {
        match error {
            nom::Err::Incomplete(needed) => HpsParseError::Incomplete(needed),
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                HpsParseError::InvalidData(e.code, e.input.into())
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum HpsDecodeError {
    #[error("One of the audio frame headers contains a coefficient index of {0} which is invalid. Length of the coefficients array is {COEFFICIENT_PAIRS_PER_CHANNEL}")]
    InvalidCoefficientIndex(usize),

    #[error("This file has {channel_count} audio channels, but the block at index {block_index} has {frame_count} frames. {frame_count} is not a multiple of {channel_count}")]
    InvalidBlockSize {
        block_index: usize,
        frame_count: usize,
        channel_count: usize,
    },
}
