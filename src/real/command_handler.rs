use clap::Parser;
use std::path::PathBuf;

/// Is this the DOC?
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Update the music database with a new (or old but with new songs)
    /// music directory.
    ///
    /// The music directory is where `museum` will search for FLAC files.
    ///
    /// If you do not call this, but no database can be found,
    /// a new one will be generated from a directory of your choice.
    /// If a database is found, it will be used.
    #[arg(short, long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    pub update: Option<PathBuf>,

    /// List *all* songs in the music database.
    #[arg(short, long)]
    pub list: bool,

    /// Add a specific song to the music database.
    #[arg(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub add: Option<PathBuf>,

    /// Add an entire directory of songs to the music database.
    #[arg(short, long, value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    pub dir_add: Option<PathBuf>,
}
