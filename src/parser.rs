use nom::{
    self,
    bytes::complete::{tag, take},
    combinator::map,
    multi::count,
    number::complete::{be_i16, be_u32, be_u8},
    sequence::tuple,
    IResult,
};

use crate::hps::{ChannelInfo, Block, Frame, DSPDecoderState};

#[inline]
pub(crate) fn parse_file_header(bytes: &[u8]) -> IResult<&[u8], (u32, u32)> {
    map(
        tuple((tag(" HALPST\0"), be_u32, be_u32)),
        |(_, sample_rate, channel_count)| (sample_rate, channel_count),
    )(bytes)
}

#[inline]
pub(crate) fn parse_channel_info(bytes: &[u8]) -> IResult<&[u8], ChannelInfo> {
    map(
        tuple((
            be_u32,
            take(4usize),
            be_u32,
            take(4usize),
            count(tuple((be_i16, be_i16)), 8),
            take(8usize), // there's a DSP decoder state here that we don't use
        )),
        |(largest_block_length, _, sample_count, _, coefficients, _)| ChannelInfo {
            largest_block_length,
            sample_count,
            coefficients,
        },
    )(bytes)
}

#[inline]
pub(crate) fn parse_block(file_size: usize) -> impl FnMut(&[u8]) -> IResult<&[u8], Block> {
    move |bytes: &[u8]| {
        let address = file_size - bytes.len();
        let (bytes, dsp_data_length) = be_u32(bytes)?;
        let frames_in_block = dsp_data_length as usize / 8;

        let (bytes, _) = take(4usize)(bytes)?;
        let (bytes, next_block_address) = be_u32(bytes)?;
        let (bytes, left_decoder_state) = parse_dsp_decoder_state(bytes)?;
        let (bytes, right_decoder_state) = parse_dsp_decoder_state(bytes)?;
        let (bytes, _) = take(4usize)(bytes)?;
        let (bytes, frames) = count(parse_frame, frames_in_block)(bytes)?;

        Ok((
            bytes,
            Block {
                address: address as u32,
                dsp_data_length,
                next_block_address,
                decoder_states: [left_decoder_state, right_decoder_state],
                frames,
            },
        ))
    }
}

#[inline]
fn parse_dsp_decoder_state(bytes: &[u8]) -> IResult<&[u8], DSPDecoderState> {
    map(
        tuple((take(1usize), take(1usize), be_i16, be_i16, take(2usize))),
        |(_, _, initial_hist_1, initial_hist_2, _)| DSPDecoderState {
            // ps_hi,
            // ps,
            initial_hist_1,
            initial_hist_2,
        },
    )(bytes)
}

#[inline]
fn parse_frame(bytes: &[u8]) -> IResult<&[u8], Frame> {
    map(
        tuple((be_u8, be_u8, be_u8, be_u8, be_u8, be_u8, be_u8, be_u8)),
        |(header, f1, f2, f3, f4, f5, f6, f7)| Frame {
            header,
            encoded_sample_data: [f1, f2, f3, f4, f5, f6, f7],
        },
    )(bytes)
}
