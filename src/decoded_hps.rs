//! Contains [`DecodedHps`] for iterating over decoded PCM samples. For looping
//! songs, this is an _infinite_ iterator. While an iterator like this is useful
//! for audio playback, you may need to access the samples directly for other
//! kinds of applications.
//!
//! # Getting a vec of PCM samples
//!
//! If you'd like to get a vec of the underlying PCM samples, use
//! [`.samples()`](DecodedHps::samples) to get the PCM samples as a slice, then
//! collect them into a vec:
//! ```
//! let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//! let audio: DecodedHps = hps.decode()?;
//!
//! let samples: Vec<i16> = audio.samples().to_vec();
//! assert_eq!(samples.len(), 6_415_472);
//! ```

use rayon::prelude::*;

#[cfg(feature = "rodio-source")]
use crate::decoded_hps_rodio_source::DecodedHpsRodioSource;
use crate::errors::HpsDecodeError;
use crate::hps::{COEFFICIENT_PAIRS_PER_CHANNEL, DSPDecoderState, Frame, Hps};

const SAMPLES_PER_FRAME: usize = 14;

/// An iterator over decoded PCM samples.
///
/// For general usage, see the [module-level documentation.](crate::decoded_hps)
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedHps {
    samples: Vec<i16>,
    current_index: usize,
    loop_sample_index: Option<usize>,
    /// Number of samples per second per audio channel
    pub sample_rate: u32,
    /// Number of audio channels
    pub channel_count: u32,
}

impl Iterator for DecodedHps {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.samples.get(self.current_index), self.loop_sample_index) {
            // If there are more samples to play, return the next one
            (Some(&sample), _) => {
                self.current_index += 1;
                Some(sample)
            }
            // If there are no more samples to play, but there's a loop_sample_index
            // return the sample at that index, and continue from there
            (None, Some(loop_sample_index)) => {
                self.current_index = loop_sample_index + 1;
                Some(self.samples[loop_sample_index])
            }
            // Otherwise, there's nothing else to play
            (None, None) => None,
        }
    }
}

impl DecodedHps {
    pub(crate) fn new(hps: &Hps) -> Result<Self, HpsDecodeError> {
        let samples = hps
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
                    &hps.channel_info[0].coefficients,
                )?;

                let right_samples = Self::decode_frames(
                    &block.frames[half_index..],
                    &block.decoder_states[1],
                    &hps.channel_info[1].coefficients,
                )?;

                // Interleave the samples with each other
                Ok(left_samples
                    .into_iter()
                    .zip(right_samples)
                    .flat_map(|(left_sample, right_sample)| [left_sample, right_sample]))
            })
            .collect::<Result<Vec<_>, HpsDecodeError>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let loop_sample_index = hps.loop_block_index.map(|index| {
            hps.blocks[..index]
                .iter()
                .map(|b| b.frames.len())
                .sum::<usize>()
                * SAMPLES_PER_FRAME
        });

        Ok(Self {
            samples,
            current_index: 0,
            loop_sample_index,
            sample_rate: hps.sample_rate,
            channel_count: hps.channel_count,
        })
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

    /// Get the underlying decoded PCM samples as a slice.
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Returns `true` if the song loops. If this is the case, it's an _infinite_ iterator.
    pub fn is_looping(&self) -> bool {
        self.loop_sample_index.is_some()
    }

    /// Returns the total duration of the song without any looping.
    pub fn duration(&self) -> std::time::Duration {
        let sample_count = self.samples.len() as u64;
        let samples_per_second = (self.sample_rate * self.channel_count) as u64;
        std::time::Duration::from_millis(1000 * sample_count / samples_per_second)
    }

    /// Converts the [`DecodedHps`] into a source that can be played by the [`rodio`](https://docs.rs/rodio/0.21.1/rodio/index.html) crate.
    #[cfg_attr(docsrs, doc(cfg(feature = "rodio-source")))]
    #[cfg(feature = "rodio-source")]
    pub fn into_rodio_source(self) -> DecodedHpsRodioSource {
        DecodedHpsRodioSource(self)
    }
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

#[inline(always)]
fn clamp_i16(val: i32) -> i16 {
    if val < (i16::MIN as i32) {
        i16::MIN
    } else if val > (i16::MAX as i32) {
        i16::MAX
    } else {
        val as i16
    }
}
