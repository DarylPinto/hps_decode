# HPS Decode

[![Latest Version][version-badge]][version-url]
[![Rust Documentation][docs-badge]][docs-url]
[![Build Status][actions-badge]][actions-url]

[version-badge]: https://img.shields.io/crates/v/hps_decode.svg
[version-url]: https://crates.io/crates/hps_decode
[docs-badge]: https://img.shields.io/badge/docs-latest-blue.svg
[docs-url]: https://docs.rs/hps_decode
[actions-badge]: https://github.com/DarylPinto/hps_decode/actions/workflows/ci.yml/badge.svg
[actions-url]: https://github.com/DarylPinto/hps_decode/actions/workflows/ci.yml

A Rust library for parsing and decoding _Super Smash Bros. Melee_ music files.

## Quick Start

Playing back a stereo `.hps` file with
[rodio.](https://docs.rs/rodio/0.17.1/rodio/index.html)

In your `Cargo.toml`:
```toml
[dependencies]
hps_decode = { version = "0.2.1", features = ["rodio-source"] }
rodio = { version = "0.17.1", default-features = false }
```

In your `main.rs`:
```rust
use hps_decode::Hps;
use rodio::{OutputStream, Sink};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Decode an .hps file into PCM samples for playback
    let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
    let audio = hps.decode()?;

    // Play the song with the rodio library
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    sink.append(audio);
    sink.play();
    sink.sleep_until_end();

    Ok(())
}
```

## Documentation

Check out [docs.rs][docs-url] for more details about the library.

## Benchmarking

This library can be benchmarked using [criterion](https://github.com/bheisler/criterion.rs) by running `cargo bench`. Reports with the results will be generated at `target/criterion/report/index.html`

## .HPS File Layout

For general purpose, language agnostic information about the `.hps` file format,
[see here.](https://github.com/DarylPinto/hps_decode/blob/main/HPS-LAYOUT.md)
