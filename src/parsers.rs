use winnow::{
    binary::{be_i16, be_u8, be_u32},
    combinator::repeat,
    error::ContextError,
    seq,
    token::{literal, take},
};

use crate::errors::HpsParseError;
use crate::hps::{Block, COEFFICIENT_PAIRS_PER_CHANNEL, ChannelInfo, DSPDecoderState, Frame};
use winnow::prelude::*;

pub(crate) fn parse_file_header(bytes: &mut &[u8]) -> Result<(u32, u32), HpsParseError> {
    let _ = literal(" HALPST\0")
        .parse_next(bytes)
        .map_err(|_: ContextError| HpsParseError::InvalidMagicNumber)?;
    let sample_rate = be_u32
        .parse_next(bytes)
        .map_err(|e: ContextError| HpsParseError::InvalidData(e))?;
    let channel_count = be_u32
        .parse_next(bytes)
        .map_err(|e: ContextError| HpsParseError::InvalidData(e))?;

    if channel_count != 2 {
        return Err(HpsParseError::UnsupportedChannelCount(channel_count));
    }

    Ok((sample_rate, channel_count))
}

pub(crate) fn parse_channel_info(bytes: &mut &[u8]) -> winnow::Result<ChannelInfo> {
    let largest_block_length = be_u32.parse_next(bytes)?;
    let _ = take(4usize).parse_next(bytes)?;
    let sample_count = be_u32.parse_next(bytes)?;
    let _ = take(4usize).parse_next(bytes)?;
    let coefficients: Vec<(i16, i16)> =
        repeat(1..=COEFFICIENT_PAIRS_PER_CHANNEL, seq!((be_i16, be_i16))).parse_next(bytes)?;
    let _dsp_decoder_state = take(8usize).parse_next(bytes)?;

    Ok(ChannelInfo {
        largest_block_length,
        sample_count,
        coefficients: coefficients.try_into().unwrap_or_else(|_| {
            // This is unreachable because the coefficients variable above
            // and ChannelInfo.coefficients both have a length of
            // COEFFICIENT_PAIRS_PER_CHANNEL
            unreachable!()
        }),
    })
}

pub(crate) fn parse_block(file_size: usize) -> impl FnMut(&mut &[u8]) -> winnow::Result<Block> {
    move |bytes: &mut &[u8]| {
        let offset = file_size - bytes.len();
        let dsp_data_length = be_u32.parse_next(bytes)?;
        let frame_count = dsp_data_length as usize / 8;

        let _ = take(4usize).parse_next(bytes)?;
        let next_block_offset = be_u32.parse_next(bytes)?;
        let left_decoder_state = parse_dsp_decoder_state(bytes)?;
        let right_decoder_state = parse_dsp_decoder_state(bytes)?;
        let _ = take(4usize).parse_next(bytes)?;
        let frames = repeat(frame_count, parse_frame).parse_next(bytes)?;

        Ok(Block {
            offset: offset as u32,
            dsp_data_length,
            next_block_offset,
            decoder_states: [left_decoder_state, right_decoder_state],
            frames,
        })
    }
}

#[inline]
fn parse_dsp_decoder_state(bytes: &mut &[u8]) -> winnow::Result<DSPDecoderState> {
    let _ps_hi = take(1usize).parse_next(bytes)?;
    let _ps = take(1usize).parse_next(bytes)?;
    let initial_hist_1 = be_i16.parse_next(bytes)?;
    let initial_hist_2 = be_i16.parse_next(bytes)?;
    let _ = take(2usize).parse_next(bytes)?;

    Ok(DSPDecoderState {
        // ps_hi,
        // ps,
        initial_hist_1,
        initial_hist_2,
    })
}

#[inline(always)]
fn parse_frame(bytes: &mut &[u8]) -> winnow::Result<Frame> {
    Ok(Frame {
        header: be_u8.parse_next(bytes)?,
        encoded_sample_data: [
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
            be_u8.parse_next(bytes)?,
        ],
    })
}
