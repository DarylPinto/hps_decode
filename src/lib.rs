#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

//! A library for parsing and decoding _Super Smash Bros. Melee_ music files.
//!
//! # Quick Start
//!
//! Decoding a stereo `.hps` file into audio and listening to it with
//! [rodio:](https://docs.rs/rodio/0.21.1/rodio/index.html)
//!
//! Install dependencies:
//! ```sh
//! cargo add rodio --no-default-features --features playback
//! cargo add hps_decode --features rodio-source
//! ```
//!
//! In your `main.rs`:
//! ```
//! use hps_decode::Hps;
//! use rodio::{OutputStreamBuilder, Sink};
//! use std::error::Error;
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Decode an .hps file into raw audio data
//!     let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//!     let audio = hps.decode()?;
//!
//!     // Play it using the rodio library
//!     let stream_handle = OutputStreamBuilder::open_default_stream()?;
//!     let sink = Sink::connect_new(&stream_handle.mixer());
//!     let source = audio.into_rodio_source();
//!
//!     sink.append(source);
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
#[cfg(feature = "rodio-source")]
pub mod decoded_hps_rodio_source;
pub mod hps;
