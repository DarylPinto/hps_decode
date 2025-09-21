use hps_decode::Hps;
use rodio::{OutputStreamBuilder, Sink};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    // Get the path of an .hps file
    let root_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let hps_file_path = PathBuf::from(root_dir).join("./test-data/test-song.hps");

    // Decode the .hps file into PCM samples for playback
    let hps: Hps = std::fs::read(hps_file_path)?.try_into()?;
    let audio = hps.decode()?;

    // Play the song with the rodio library
    let stream_handle = OutputStreamBuilder::open_default_stream()?;
    let sink = Sink::connect_new(&stream_handle.mixer());

    sink.append(audio);
    sink.play();
    sink.sleep_until_end();

    Ok(())
}
