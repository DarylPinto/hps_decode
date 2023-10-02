//! Contains [`Hps`] for representing the contents of an `.hps` file in a structured format.
//!
//! To parse raw binary data from an `.hps` file into an [`Hps`], you can use `.try_into()`:
//!
//! ```
//! let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//! assert_eq!(hps.sample_rate, 32_000);
//! assert_eq!(hps.channel_count, 2);
//! ```
//!
//! # Decoding into audio
//! To decode an [`Hps`] into audio, you can use the [`.decode()`](Hps::decode)
//! method:
//! ```
//! let audio = hps.decode()?;
//!
//! // For looping songs, this will go on forever:
//! for sample in audio {
//!     println!("{sample}");
//! }
//! ```
//! If you’d like to get the underlying PCM samples as a vec, check out the
//! [`decoded_hps`](crate::decoded_hps) module.

use std::collections::HashSet;

use nom::multi::many0;
use rayon::prelude::*;

use crate::decoded_hps::DecodedHps;
use crate::errors::{HpsDecodeError, HpsParseError};
use crate::parsers::{parse_block, parse_channel_info, parse_file_header};

const DSP_BLOCK_SECTION_OFFSET: u32 = 0x80;
pub(crate) const SAMPLES_PER_FRAME: usize = 14;
pub(crate) const COEFFICIENT_PAIRS_PER_CHANNEL: usize = 8;

/// A container for HPS file data.
///
/// For general usage, see the [module-level documentation.](crate::hps)
#[derive(Debug, Clone, PartialEq)]
pub struct Hps {
    /// Number of samples per second per audio channel
    pub sample_rate: u32,
    /// Number of audio channels
    pub channel_count: u32,
    /// Information about the audio channels
    pub channel_info: [ChannelInfo; 2],
    /// DSP Block data
    pub blocks: Vec<Block>,
    /// Index of the block to loop back to when the track ends. `None` if the track doesn't loop
    pub loop_block_index: Option<usize>,
}

impl TryFrom<&[u8]> for Hps {
    type Error = HpsParseError;

    /// Create an `Hps` from a byte slice
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let file_size = bytes.len();

        // File Header
        let (bytes, (sample_rate, channel_count)) = parse_file_header(bytes)?;

        // Left and Right Channel Information
        let (bytes, left_channel_info) = parse_channel_info(bytes)?;
        let (bytes, right_channel_info) = parse_channel_info(bytes)?;

        // Parse the rest of the file as DSP blocks
        let (_, mut blocks) = many0(parse_block(file_size))(bytes)?;

        // Remove any blocks whose `offset` is not referenced by any other
        // blocks' `next_block_offset`
        //
        // This is specifically to remove any blocks that might have been
        // accidentally parsed from garbage data. While it's extremely unlikely
        // to occur in a real HPS file, better safe than sorry.
        let valid_block_offsets = std::iter::once(DSP_BLOCK_SECTION_OFFSET)
            .chain(blocks.iter().map(|b| b.next_block_offset))
            .collect::<HashSet<_>>();
        blocks.retain(|b| valid_block_offsets.contains(&b.offset));

        let loop_block_index = blocks.last().and_then(|last_block| {
            blocks
                .iter()
                .position(|block| block.offset == last_block.next_block_offset)
        });

        Ok(Hps {
            sample_rate,
            channel_count,
            channel_info: [left_channel_info, right_channel_info],
            blocks,
            loop_block_index,
        })
    }
}

impl TryFrom<Vec<u8>> for Hps {
    type Error = HpsParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<&Vec<u8>> for Hps {
    type Error = HpsParseError;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl Hps {
    /// Decode an [`Hps`] into audio. See the [module-level
    /// documentation](crate::hps) for more information.
    pub fn decode(&self) -> Result<DecodedHps, HpsDecodeError> {
        let samples = self
            .blocks
            .par_iter()
            .map(|block| {
                // The first half of the frames in the block are for the left
                // audio channel, and the other half are for the right
                let half_index = block.frames.len() / 2;

                // Decode the samples for the left and right audio channels
                let left_samples = Self::decode_frames(
                    &block.frames[..half_index],
                    &block.decoder_states[0],
                    &self.channel_info[0].coefficients,
                )?;

                let right_samples = Self::decode_frames(
                    &block.frames[half_index..],
                    &block.decoder_states[1],
                    &self.channel_info[1].coefficients,
                )?;

                // Interleave the samples with each other
                Ok(left_samples
                    .into_iter()
                    .zip(right_samples)
                    .flat_map(|(left_sample, right_sample)| [left_sample, right_sample])
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<Vec<_>>, HpsDecodeError>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        Ok(DecodedHps::new(self, samples))
    }

    /// Decode a slice of DSP block frames into samples
    fn decode_frames(
        frames: &[Frame],
        decoder_state: &DSPDecoderState,
        coefficients: &[(i16, i16)],
    ) -> Result<Vec<i16>, HpsDecodeError> {
        let sample_count = frames.len() * SAMPLES_PER_FRAME;
        let mut samples: Vec<i16> = Vec::with_capacity(sample_count);

        let mut hist1 = decoder_state.initial_hist_1;
        let mut hist2 = decoder_state.initial_hist_2;

        for frame in frames {
            let scale = 1 << (frame.header & 0xF);
            let coef_index = (frame.header >> 4) as usize;
            if coef_index >= COEFFICIENT_PAIRS_PER_CHANNEL {
                return Err(HpsDecodeError::InvalidCoefficientIndex(coef_index));
            }
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
        }

        Ok(samples)
    }
}

/// Information about an audio channel. Notably, an audio channel contains 16
/// "coefficients" that are used in the calculation to decode samples.
#[derive(Debug, PartialEq, Clone)]
pub struct ChannelInfo {
    pub largest_block_length: u32,
    pub sample_count: u32,
    pub coefficients: [(i16, i16); COEFFICIENT_PAIRS_PER_CHANNEL],
}

/// The audio data contained in an [`Hps`] is split into multiple "blocks", each
/// containing [`Frame`]s of encoded samples as well as a link to the start of the
/// next block.
///
/// In a stereo [`Hps`], the first half of the frames in each block are for the
/// left audio channel, and other half are for the right.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub offset: u32,
    pub dsp_data_length: u32,
    pub next_block_offset: u32,
    pub decoder_states: [DSPDecoderState; 2],
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DSPDecoderState {
    // ps_hi: u8, // unused?
    // ps: u8,    // unused?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_blocks_correctly() {
        let hps: Hps = std::fs::read("test-data/test-song.hps")
            .unwrap()
            .try_into()
            .unwrap();
        let decoded_bytes = hps
            .decode()
            .unwrap()
            .samples()
            .into_iter()
            .flat_map(|sample| sample.to_be_bytes())
            .collect::<Vec<_>>();

        // // Create a new binary file of decoded samples for testing
        // use std::io::prelude::*;
        // let mut file = std::fs::File::create("new.bin").unwrap();
        // file.write_all(&decoded_bytes).unwrap();

        let expected_bytes = std::fs::read("test-data/test-song-decoded.bin").unwrap();
        assert_eq!(expected_bytes, decoded_bytes);
    }

    #[test]
    fn doesnt_include_any_blocks_more_than_once() {
        let hps: Hps = std::fs::read("test-data/test-song.hps")
            .unwrap()
            .try_into()
            .unwrap();
        let block_count = hps.blocks.len();
        let unique_block_offsets = hps
            .blocks
            .iter()
            .map(|block| block.offset)
            .collect::<HashSet<_>>();
        let unique_block_count = unique_block_offsets.len();
        assert_eq!(block_count, unique_block_count);
    }

    #[test]
    fn parses_last_block_even_if_its_very_short() {
        let hps: Hps = std::fs::read("test-data/short-last-block-with-loop.hps")
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(hps.blocks.len(), 8);
        assert!(hps.loop_block_index.is_some());
    }

    #[test]
    fn properly_handles_invalid_coefficient_index() {
        let hps: Hps = std::fs::read("test-data/corrupt-dsp-frame-header.hps")
            .unwrap()
            .try_into()
            .unwrap();

        assert!(matches!(
            hps.decode().unwrap_err(),
            HpsDecodeError::InvalidCoefficientIndex(..)
        ));
    }

    #[test]
    fn reads_metadata_correctly() {
        let hps: Hps = std::fs::read("test-data/test-song.hps")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(hps.sample_rate, 32000);
        assert_eq!(hps.channel_count, 2);
        assert_eq!(
            hps.channel_info[0],
            ChannelInfo {
                largest_block_length: 65536,
                sample_count: 2874134,
                coefficients: [
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
                coefficients: [
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
