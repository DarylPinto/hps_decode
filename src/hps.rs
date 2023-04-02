//! Contains [`Hps`] for representing the contents of an `.hps` file in a structured format.
//!
//! To extract the data from an `.hps` binary file into an [`Hps`], you can use `.try_into()` on a byte slice:
//!
//! ```
//! let bytes = std::fs::read("./respect-your-elders.hps").unwrap();
//! let hps: Hps = bytes.as_slice().try_into().unwrap();
//!
//! assert_eq!(hps.sample_rate, 32_000);
//! assert_eq!(hps.channel_count, 2);
//! ```
//!
//! # Decoding into PCM samples
//! To decode an [`Hps`] into PCM samples, you can use the [`decode`](Hps::decode) method:
//! ```
//! let samples: Vec<i16> = hps.decode();
//! assert_eq!(samples.len(), 6_415_472);
//! ```
//!
//! If you'd like to get an _infinite_ iterator for a looping song, take a look at the
//! [pcm_iterator](crate::pcm_iterator) module.

use rayon::prelude::*;
use thiserror::Error;

const FILE_HEADER_OFFSET: usize = 0;
const LEFT_CHANNEL_OFFSET: usize = 0x10;
const RIGHT_CHANNEL_OFFSET: usize = 0x48;
const DSP_BLOCK_SECTION_OFFSET: usize = 0x80;

/// A conatiner for HPS file data.
///
/// For general usage, see the [module-level documentation.](crate::hps)
#[derive(Debug, Clone, PartialEq)]
pub struct Hps {
    /// Number of samples per second per audio channel
    pub sample_rate: u32,
    /// Number of audio channels
    pub channel_count: u32,
    /// Information about the audio channel(s)
    pub channel_info: Vec<ChannelInfo>,
    /// DSP Block data
    pub blocks: Vec<Block>,
    /// Index of the block to loop back to when the track ends. `None` if the track doesn't loop
    pub loop_block_index: Option<usize>,
}

#[derive(Error, Debug)]
pub enum HpsParseError {
    /// The first 8 bytes in the file are not ` HALPST\0`
    #[error("Invalid magic number. Expected ' HALPST\0'")]
    InvalidMagicNumber,
    /// The number of audio channels in the provided file is not suppored by the library
    #[error("Only stereo is currently supported, but the provided file has {0} audio channel(s)")]
    UnsupportedChannelCount(u32),
}

impl TryFrom<&[u8]> for Hps {
    type Error = HpsParseError;

    /// Create an `Hps` from a byte slice
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        // Ensure magic number is present
        if &bytes[..0x08] != b" HALPST\0" {
            return Err(Self::Error::InvalidMagicNumber);
        }

        let channel_count = read_u32(bytes, FILE_HEADER_OFFSET + 0x0C);
        let sample_rate = read_u32(bytes, FILE_HEADER_OFFSET + 0x08);

        // Ensure provided file has two audio channels
        if channel_count != 2 {
            return Err(Self::Error::UnsupportedChannelCount(channel_count));
        }

        // Read the channel info in `bytes` at `offset`
        let read_channel_info_at = |offset: usize| ChannelInfo {
            largest_block_length: read_u32(bytes, offset),
            sample_count: read_u32(bytes, offset + 0x08),
            coefficients: (0x10..0x30)
                .step_by(4)
                .map(|step| {
                    (
                        read_i16(bytes, offset + step),
                        read_i16(bytes, offset + step + 2),
                    )
                })
                .collect(),
        };

        // Read the DSP block data in `bytes` at `block_start_address`
        // Returns the block as well as the address of the next block
        let read_block_at = |block_start_address: usize| -> (Block, usize) {
            let address = block_start_address as u32;
            let dsp_data_length = read_u32(bytes, block_start_address);
            let next_block_address = read_u32(bytes, block_start_address + 0x08);

            let frames = ((block_start_address + 0x20)
                ..(block_start_address + 0x20 + dsp_data_length as usize))
                .step_by(8)
                .filter_map(|frame_offset| {
                    Some(Frame {
                        header: bytes[frame_offset],
                        encoded_sample_data: bytes[(frame_offset + 1)..(frame_offset + 8)]
                            .try_into()
                            .ok()?,
                    })
                })
                .collect::<Vec<_>>();

            let read_decoder_state_at = |ds_offset: usize| DSPDecoderState {
                ps_hi: bytes[block_start_address + ds_offset],
                ps: bytes[block_start_address + ds_offset + 1],
                initial_hist_1: read_i16(bytes, block_start_address + ds_offset + 2),
                initial_hist_2: read_i16(bytes, block_start_address + ds_offset + 4),
            };

            let left_decoder_state = read_decoder_state_at(0x0C);
            let right_decoder_state = read_decoder_state_at(0x14);

            let block = Block {
                address,
                dsp_data_length,
                next_block_address,
                decoder_states: vec![left_decoder_state, right_decoder_state],
                frames,
            };

            (block, next_block_address as usize)
        };

        // Left and Right Channel Information
        let left_channel_info = read_channel_info_at(LEFT_CHANNEL_OFFSET);
        let right_channel_info = read_channel_info_at(RIGHT_CHANNEL_OFFSET);

        // DSP Blocks
        let largest_block_length = std::cmp::max(
            left_channel_info.largest_block_length as usize,
            right_channel_info.largest_block_length as usize,
        );
        let block_count = ((bytes.len() - DSP_BLOCK_SECTION_OFFSET) / largest_block_length) + 1;
        let blocks = (0..block_count)
            .scan(DSP_BLOCK_SECTION_OFFSET, |next_block_address, _| {
                let (block, next) = read_block_at(*next_block_address);
                *next_block_address = next;
                Some(block)
            })
            .collect::<Vec<_>>();

        // Index of the block to loop back to when the track ends
        let loop_block_index = blocks.last().and_then(|last_block| {
            blocks
                .iter()
                .position(|block| block.address == last_block.next_block_address)
        });

        // Final HPS structure
        Ok(Self {
            channel_count,
            sample_rate,
            channel_info: vec![left_channel_info, right_channel_info],
            blocks,
            loop_block_index,
        })
    }
}

impl Hps {
    /// Decode an [`Hps`] into a vector of PCM samples. If you'd like to get an
    /// _infinite_ iterator for a looping song, take a look at the [pcm_iterator](crate::pcm_iterator) module.
    pub fn decode(&self) -> Vec<i16> {
        self.blocks
            .par_iter()
            .flat_map(|block| {
                // The first half of the frames in the block are for the left
                // audio channel, and the other half are for the right
                let half_index = block.frames.len() / 2;

                // Decode the samples for the left and right audio channels
                let left_samples = Self::decode_frames(
                    &block.frames[..half_index],
                    &block.decoder_states[0],
                    &self.channel_info[0].coefficients,
                );
                let right_samples = Self::decode_frames(
                    &block.frames[half_index..],
                    &block.decoder_states[1],
                    &self.channel_info[1].coefficients,
                );

                // Interleave the samples with each other
                left_samples
                    .into_iter()
                    .zip(right_samples)
                    .flat_map(|(left_sample, right_sample)| [left_sample, right_sample])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// Decode a slice of DSP block frames into samples
    fn decode_frames(
        frames: &[Frame],
        decoder_state: &DSPDecoderState,
        coefficients: &[(i16, i16)],
    ) -> Vec<i16> {
        let sample_count = frames.len() * 14;
        let mut samples: Vec<i16> = Vec::with_capacity(sample_count);

        let mut hist1 = decoder_state.initial_hist_1;
        let mut hist2 = decoder_state.initial_hist_2;

        frames.iter().for_each(|frame| {
            let scale = 1 << (frame.header & 0xF);
            let coef_index = (frame.header >> 4) as usize;
            let (coef1, coef2) = coefficients[coef_index];

            frame
                .encoded_sample_data
                .iter()
                .flat_map(|&byte| [get_high_nibble(byte), get_low_nibble(byte)])
                .for_each(|nibble| {
                    let sample = clamp_i16(
                        (((nibble as i32 * scale) << 11)
                            + 1024
                            + (coef1 as i32 * hist1 as i32 + coef2 as i32 * hist2 as i32))
                            >> 11,
                    );

                    hist2 = hist1;
                    hist1 = sample;
                    samples.push(sample);
                });
        });

        samples
    }
}

/// Information about an audio channel. Notably, an audio channel contains 16
/// "coefficients" that are used in the calculation to decode samples.
#[derive(Debug, PartialEq, Clone)]
pub struct ChannelInfo {
    pub largest_block_length: u32,
    pub sample_count: u32,
    pub coefficients: Vec<(i16, i16)>,
}
/// The audio data contained in an [`Hps`] is split into multiple "blocks", each
/// containing [`Frame`]s of encdoded samples as well as a link to the start of the
/// next block.
///
/// In a stereo [`Hps`], the first half of the frames in each block are for the
/// left audio channel, and other half are for the right.
///
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub address: u32,
    pub dsp_data_length: u32,
    pub next_block_address: u32,
    pub decoder_states: Vec<DSPDecoderState>,
    // pub right_decoder_state: DSPDecoderState,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DSPDecoderState {
    ps_hi: u8, // unused?
    ps: u8,    // unused?
    pub initial_hist_1: i16,
    pub initial_hist_2: i16,
}

/// Each frame of audio data contains 14 encoded PCM samples.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub header: u8,
    pub encoded_sample_data: [u8; 7],
}

static NIBBLE_TO_I8: [i8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, -8, -7, -6, -5, -4, -3, -2, -1];

#[inline(always)]
fn get_low_nibble(byte: u8) -> i8 {
    NIBBLE_TO_I8[(byte & 0xF) as usize]
}

#[inline(always)]
fn get_high_nibble(byte: u8) -> i8 {
    NIBBLE_TO_I8[((byte >> 4) & 0xF) as usize]
}

fn clamp_i16(val: i32) -> i16 {
    if val < (i16::MIN as i32) {
        i16::MIN
    } else if val > (i16::MAX as i32) {
        i16::MAX
    } else {
        val as i16
    }
}

fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    let size = (i16::BITS / 8) as usize;
    let end: usize = offset + size;
    i16::from_be_bytes(
        bytes[offset..end]
            .try_into()
            .unwrap_or_else(|_| unreachable!()),
    )
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    let size = (u32::BITS / 8) as usize;
    let end: usize = offset + size;
    u32::from_be_bytes(
        bytes[offset..end]
            .try_into()
            .unwrap_or_else(|_| unreachable!()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn decodes_blocks_correctly() {
        let song = std::fs::read("test-data/test-song.hps").unwrap();
        let hps = Hps::try_from(song.as_slice()).unwrap();
        let decoded = hps.decode();
        let decoded_bytes = decoded
            .iter()
            .flat_map(|sample| sample.to_be_bytes())
            .collect::<Vec<_>>();

        // // Create a new binary file of decoded samples for testing
        // use std::io::prelude::*;
        // let mut file = std::fs::File::create("bin.bin").unwrap();
        // file.write_all(&decoded_bytes).unwrap();

        let expected_bytes = std::fs::read("test-data/test-song-decoded.bin").unwrap();
        assert_eq!(expected_bytes, decoded_bytes);
    }

    #[test]
    fn doesnt_include_any_blocks_more_than_once() {
        let song = std::fs::read("test-data/test-song.hps").unwrap();
        let hps = Hps::try_from(song.as_slice()).unwrap();
        let block_count = hps.blocks.len();
        let unique_block_start_addresses = hps
            .blocks
            .iter()
            .map(|block| block.address)
            .collect::<HashSet<_>>();
        let unique_block_count = unique_block_start_addresses.len();
        assert_eq!(block_count, unique_block_count);
    }

    #[test]
    fn reads_metadata_correctly() {
        let song = std::fs::read("test-data/test-song.hps").unwrap();
        let hps = Hps::try_from(song.as_slice()).unwrap();
        assert_eq!(hps.sample_rate, 32000);
        assert_eq!(hps.channel_count, 2);
        assert_eq!(
            hps.channel_info[0],
            ChannelInfo {
                largest_block_length: 65536,
                sample_count: 2874134,
                coefficients: vec![
                    (492, -294),
                    (2389, -1166),
                    (1300, 135),
                    (3015, -1133),
                    (1491, -717),
                    (2845, -1208),
                    (1852, -11),
                    (3692, -1705),
                ],
            },
        );
        assert_eq!(
            hps.channel_info[1],
            ChannelInfo {
                largest_block_length: 65536,
                sample_count: 2874134,
                coefficients: vec![
                    (411, -287),
                    (2359, -1100),
                    (1247, 143),
                    (3147, -1288),
                    (1472, -773),
                    (2600, -894),
                    (1745, 93),
                    (3703, -1715),
                ],
            }
        );
    }

    #[test]
    fn expects_halpst_header() {
        let bytes = b"hello world";
        let error = Hps::try_from(bytes.as_slice()).unwrap_err();
        assert!(matches!(error, HpsParseError::InvalidMagicNumber));
    }
}
