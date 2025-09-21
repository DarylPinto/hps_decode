//! A library for parsing and decoding _Super Smash Bros. Melee_ music files.
//!
//! # Quick Start
//!
//! Decoding a stereo `.hps` file into audio and listening to it with
//! [rodio:](https://docs.rs/rodio/0.21.1/rodio/index.html)
//!
//! Install dependencies:
//! ```sh
//! cargo add rodio hps_decode --no-default-features --features "rodio/playback hps_decode/rodio-source"
//! ```
//!
//! In your `main.rs`:
//! ```
//! use hps_decode::Hps;
//! use rodio::{OutputStreamBuilder, Sink};
//! use std::error::Error;
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Decode an .hps file into PCM samples for playback
//!     let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//!     let audio = hps.decode()?;
//!
//!     // Play the song with the rodio library
//!     let stream_handle = OutputStreamBuilder::open_default_stream()?;
//!     let sink = Sink::connect_new(&stream_handle.mixer());
//!
//!     sink.append(audio);
//!     sink.play();
//!     sink.sleep_until_end();
//!
//!     Ok(())
//! }
//! ```
//!
//! # .HPS File Layout
//! For general purpose, language agnostic documentation of the `.hps` file format,
//! [see here.](https://github.com/DarylPinto/hps_decode/blob/main/HPS-LAYOUT.md)

mod errors;
mod parsers;

pub use hps::Hps;

pub mod decoded_hps;
pub mod hps;
