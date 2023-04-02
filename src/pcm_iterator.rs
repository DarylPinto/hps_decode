//! Contains [`PcmIterator`] for iterating over decoded PCM samples. For looping songs, this is an _infinite_ iterator.
//!
//! To create a [`PcmIterator`] from an [`Hps`], you can use `.into()`:
//! ```
//! let bytes = std::fs::read("./respect-your-elders.hps").unwrap();
//! let hps: Hps = bytes.as_slice().try_into().unwrap();
//! let pcm: PcmIterator = hps.into();
//! ```

use crate::hps::Hps;

/// An iterator over decoded PCM samples.
///
/// For general usage, see the [module-level documentation.](crate::pcm_iterator)
pub struct PcmIterator {
    samples: Vec<i16>,
    current_index: usize,
    loop_sample_index: Option<usize>,
    pub sample_rate: u32,
    pub channel_count: u32,
}

impl Iterator for PcmIterator {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Start iterating on the actual samples
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

impl From<Hps> for PcmIterator {
    fn from(hps: Hps) -> Self {
        let samples = hps.decode();

        let loop_sample_index = hps.loop_block_index.map(|index| {
            hps.blocks[0..index]
                .iter()
                .map(|b| b.frames.len() * 14)
                .sum::<usize>()
        });

        Self {
            samples,
            current_index: 0,
            loop_sample_index,
            sample_rate: hps.sample_rate,
            channel_count: hps.channel_count,
        }
    }
}
