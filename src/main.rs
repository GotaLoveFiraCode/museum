use color_eyre::eyre::{ensure, Result, WrapErr};
use etcetera::BaseStrategy;
use owo_colors::OwoColorize;

/// Code related to Songs specifically.
mod song;

/// Code related to real stuff.
/// This means:
/// - handling command-line arguments,
/// - finding files,
/// - converting files into non-real types (`Song`),
/// - etc.
mod real;

/// Interact with the `SQLite` database.
mod db;

use clap::Parser;
use real::command_handler::Cli;

fn main() -> Result<()> {
    // lol
    color_eyre::install().wrap_err("Failed to install error handling with `color-eyre`!")?;

    // Arguments.
    let cli = Cli::parse();

    // Find system application data location.
    let data_dir = etcetera::choose_base_strategy()
        .wrap_err("Failed to set `etcetera`’s strategy.")?
        .data_dir();

    // Un-initialized connection to DB.
    let conn: rusqlite::Connection;

    // If user gave new music_dir:
    if let Some(path) = cli.update {
        // Make sure argument is valid.
        let music_dir = real::gatekeeper(&path).wrap_err_with(|| {
            format!("Failed condition for: argument music directory `{path:?}`!")
        })?;

        println!(":: {} {}…", "Creating new database for".green(), path.display().blue());

        println!(":: {}…", "Searching for music".yellow());
        let files = real::find_music(&music_dir).wrap_err_with(|| {
            format!("Failed to find music files with `fd` from {music_dir:?}!")
        })?;
        // TODO: support multiple music file formats.
        println!("==> {} flac files found!", files.len().green().bold());

        println!(
            ":: {}… {}",
            "Starting to catalogue music in SQLite".yellow(),
            "This may take a while!".red().bold()
        );
        conn = db::init(&files, &data_dir).wrap_err("Failed to initialize SQLite database.")?;
        println!("==> {}", "Music catalogue complete!".green());
    } else {
        println!(":: {}…", "Checking for existing music database".yellow());
        ensure!(!data_dir.join("museum/music.db3").exists(), "…");
        println!("==> {}", "Existing database found!".green());

        conn = db::connect(&data_dir)?;
    }

    println!(":: {}…", "Displaying catalogued songs in database".yellow());
    // TODO: `retrieve_song_obj()`.
    let songs = db::retrieve_songs_vec(&conn)
        .wrap_err_with(|| format!("Failed to retrieve songs from `{conn:?}`."))?;
    for song in songs {
        println!("==> Found \"{}\"", song.path.blue());
    }

    println!(":: {}", "THAT’S ALL, FOLKS!".green().bold());

    Ok(())
}
