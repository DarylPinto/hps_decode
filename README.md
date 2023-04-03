# HPS Decode

![CI](https://github.com/DarylPinto/hps_decode/actions/workflows/ci.yml/badge.svg)

A Rust library for decoding _Super Smash Bros. Melee_ music files.

## Quick Start

Here is a quick example of how to play a stereo `.hps` file using [rodio 0.17](https://docs.rs/rodio/0.17.1/rodio/index.html):

```rs
use hps_decode::{hps::Hps, pcm_iterator::PcmIterator};
use rodio::Source;

fn main() {
    // Decode an .hps file into PCM samples for playback
    let bytes = std::fs::read("./respect-your-elders.hps").unwrap();
    let hps: Hps = bytes.as_slice().try_into().unwrap();
    let pcm: PcmIterator = hps.into();

    // Play the song with the rodio library
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let source = MySource(pcm);
    stream_handle.play_raw(source.convert_samples()).unwrap();

    // Rodio plays sound in a separate audio thread,
    // so we need to keep the main thread alive while it's playing.
    std::thread::sleep(std::time::Duration::from_secs(120));
}

// This wrapper allows us to implement `rodio::Source`
struct MySource(PcmIterator);

impl Iterator for MySource {
    type Item = i16;
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}

impl rodio::Source for MySource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { self.0.channel_count as u16 }
    fn sample_rate(&self) -> u32 { self.0.sample_rate }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}
```

## .HPS File Layout

For general purpose, language agnostic documentation of the `.hps` file format,
[see here.](https://github.com/DarylPinto/hps_decode/blob/main/HPS-LAYOUT.md)
