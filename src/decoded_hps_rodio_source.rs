//! Contains [`DecodedHpsRodioSource`] which can be used to play the song using the [`rodio`](https://docs.rs/rodio/0.21.1/rodio/index.html) crate.
//!
//! # Converting a sound into a rodio source
//!
//! To play the song using rodio, first convert it into a source using [`.into_rodio_source()`](DecodedHps::into_rodio_source), then
//! pass it to your sink:
//! ```
//! let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//! let audio: DecodedHps = hps.decode()?;
//!
//! let stream_handle = OutputStreamBuilder::open_default_stream()?;
//! let sink = Sink::connect_new(&stream_handle.mixer());
//! let source = audio.into_rodio_source();
//!
//! sink.append(source);
//! sink.play();
//! sink.sleep_until_end();
//! ```

use crate::decoded_hps::DecodedHps;

/// An source that can be played using the [`rodio`](https://docs.rs/rodio/0.21.1/rodio/index.html) crate.
///
/// For general usage, see the [module-level documentation.](crate::decoded_hps_rodio_source)
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedHpsRodioSource(DecodedHps);

impl DecodedHpsRodioSource {
    pub(crate) fn new(decoded_hps: DecodedHps) -> Self {
        Self(decoded_hps)
    }
}

impl Iterator for DecodedHpsRodioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|sample| sample as f32 / i16::MAX as f32)
    }
}

impl rodio::Source for DecodedHpsRodioSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.0.channel_count as u16
    }
    fn sample_rate(&self) -> u32 {
        self.0.sample_rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        if self.0.is_looping() {
            None
        } else {
            Some(self.0.duration())
        }
    }
}
