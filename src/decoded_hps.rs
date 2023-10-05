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

use crate::hps::{Hps, SAMPLES_PER_FRAME};

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
    pub(crate) fn new(hps: &Hps, samples: Vec<i16>) -> Self {
        let loop_sample_index = hps.loop_block_index.map(|index| {
            hps.blocks[..index]
                .iter()
                .map(|b| b.frames.len())
                .sum::<usize>()
                * SAMPLES_PER_FRAME
        });

        Self {
            samples,
            current_index: 0,
            loop_sample_index,
            sample_rate: hps.sample_rate,
            channel_count: hps.channel_count,
        }
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
}

#[cfg(feature = "rodio-source")]
impl rodio::Source for DecodedHps {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channel_count as u16
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        if self.is_looping() {
            None
        } else {
            Some(self.duration())
        }
    }
}
