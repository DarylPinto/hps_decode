//! A library for parsing and decoding _Super Smash Bros. Melee_ music files.
//!
//! # Quick Start
//!
//! Playing back a stereo `.hps` file with
//! [rodio.](https://docs.rs/rodio/0.17.1/rodio/index.html)
//!
//! In your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! hps_decode = { version = "0.2.1", features = ["rodio-source"] }
//! rodio = { version = "0.17.1", default-features = false }
//! ```
//!
//! In your `main.rs`:
//! ```
//! use hps_decode::Hps;
//! use rodio::{OutputStream, Sink};
//! use std::error::Error;
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Decode an .hps file into PCM samples for playback
//!     let hps: Hps = std::fs::read("./respect-your-elders.hps")?.try_into()?;
//!     let audio = hps.decode()?;
//!
//!     // Play the song with the rodio library
//!     let (_stream, stream_handle) = OutputStream::try_default()?;
//!     let sink = Sink::try_new(&stream_handle)?;
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
