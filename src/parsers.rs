use nom::{
    self,
    bytes::complete::{tag, take},
    combinator::map,
    multi::count,
    number::complete::{be_i16, be_u32, be_u8},
    sequence::tuple,
    IResult,
};

use crate::errors::{HpsParseError, NomByteInputError};
use crate::hps::{Block, ChannelInfo, DSPDecoderState, Frame, COEFFICIENT_PAIRS_PER_CHANNEL};

pub(crate) fn parse_file_header(bytes: &[u8]) -> Result<(&[u8], (u32, u32)), HpsParseError> {
    use HpsParseError::*;

    let (bytes, _) =
        tag(" HALPST\0")(bytes).map_err(|_: NomByteInputError<'_>| InvalidMagicNumber)?;
    let (bytes, sample_rate) = be_u32(bytes)?;
    let (bytes, channel_count) = be_u32(bytes)?;

    if channel_count != 2 {
        return Err(UnsupportedChannelCount(channel_count));
    }

    Ok((bytes, (sample_rate, channel_count)))
}

pub(crate) fn parse_channel_info(bytes: &[u8]) -> IResult<&[u8], ChannelInfo> {
    let (bytes, largest_block_length) = be_u32(bytes)?;
    let (bytes, _) = take(4usize)(bytes)?;
    let (bytes, sample_count) = be_u32(bytes)?;
    let (bytes, _) = take(4usize)(bytes)?;
    let (bytes, coefficients) =
        count(tuple((be_i16, be_i16)), COEFFICIENT_PAIRS_PER_CHANNEL)(bytes)?;
    let (bytes, _dsp_decoder_state) = take(8usize)(bytes)?;

    Ok((
        bytes,
        ChannelInfo {
            largest_block_length,
            sample_count,
            coefficients: coefficients.try_into().unwrap_or_else(|_| {
                // This is unreachable because the coefficients variable above
                // and ChannelInfo.coefficients both have a length of
                // CHANNEL_COEFFICIENT_PAIR_COUNT
                unreachable!()
            }),
        },
    ))
}

pub(crate) fn parse_block(file_size: usize) -> impl FnMut(&[u8]) -> IResult<&[u8], Block> {
    move |bytes: &[u8]| {
        let offset = file_size - bytes.len();
        let (bytes, dsp_data_length) = be_u32(bytes)?;
        let frame_count = dsp_data_length as usize / 8;

        let (bytes, _) = take(4usize)(bytes)?;
        let (bytes, next_block_offset) = be_u32(bytes)?;
        let (bytes, left_decoder_state) = parse_dsp_decoder_state(bytes)?;
        let (bytes, right_decoder_state) = parse_dsp_decoder_state(bytes)?;
        let (bytes, _) = take(4usize)(bytes)?;
        let (bytes, frames) = count(parse_frame, frame_count)(bytes)?;

        Ok((
            bytes,
            Block {
                offset: offset as u32,
                dsp_data_length,
                next_block_offset,
                decoder_states: [left_decoder_state, right_decoder_state],
                frames,
            },
        ))
    }
}

#[inline]
fn parse_dsp_decoder_state(bytes: &[u8]) -> IResult<&[u8], DSPDecoderState> {
    let (bytes, _ps_hi) = take(1usize)(bytes)?;
    let (bytes, _ps) = take(1usize)(bytes)?;
    let (bytes, initial_hist_1) = be_i16(bytes)?;
    let (bytes, initial_hist_2) = be_i16(bytes)?;
    let (bytes, _) = take(2usize)(bytes)?;

    Ok((
        bytes,
        DSPDecoderState {
            // ps_hi,
            // ps,
            initial_hist_1,
            initial_hist_2,
        },
    ))
}

#[inline(always)]
fn parse_frame(bytes: &[u8]) -> IResult<&[u8], Frame> {
    map(
        tuple((be_u8, be_u8, be_u8, be_u8, be_u8, be_u8, be_u8, be_u8)),
        |(header, s0, s1, s2, s3, s4, s5, s6)| Frame {
            header,
            encoded_sample_data: [s0, s1, s2, s3, s4, s5, s6],
        },
    )(bytes)
}
