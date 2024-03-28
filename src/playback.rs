use color_eyre::eyre::Result;
use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};

pub fn test() -> Result<()> {
    // Get output stream → default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    // Load sound from file, path is relative to Cargo.toml
    // Gitignored. Please supply your own file ;)
    let file = BufReader::new(File::open("test/music.flac")?);

    // Decode that sound file into a source
    let source = Decoder::new(file)?;
    sink.append(source);

    // Play the sound directly on the device
    sink.sleep_until_end();

    Ok(())
}


