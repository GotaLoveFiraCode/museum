use color_eyre::eyre::{ensure, Result, WrapErr};
use etcetera::BaseStrategy;
use owo_colors::OwoColorize;

use rusqlite::Connection;
use std::path::Path;

/// Code related to Songs/algo specifically.
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

// All output-related functions are in `main`. All helper functions that are not strictly related
// to the algorithm or database are in `real`. Are algorithm functions et al. are in `song`, and
// all database functions et al are in `db`.

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
    let conn: Connection;

    // If user gave new music_dir:
    if let Some(path) = cli.update {
        conn = update_db(&path, &data_dir)
            .wrap_err_with(|| format!("Failed to update DB for {}!", path.display()))?;
    } else {
        println!(":: {}…", "Checking for existing music database".yellow());
        ensure!(
            data_dir.join("museum/music.db3").exists(),
            "No previous database found! Run `museum --help`."
        );
        println!(
            "==> {} {}",
            "Existing database found!".green(),
            "Use `-l` to list songs".italic()
        );

        conn = db::connect(&data_dir)?;
    }

    if cli.list {
        println!(":: {}…", "Displaying catalogued songs in database".yellow());
        // TODO: `retrieve_song_obj()`.
        let songs = db::retrieve_songs_vec(&conn)
            .wrap_err_with(|| format!("Failed to retrieve songs from `{conn:?}`."))?;
        for song in songs {
            println!("==> Found \"{}\"", song.path.blue());
        }
    }

    println!(":: {}", "THAT’S ALL, FOLKS!".green().bold());
    Ok(())
}

/// Delete old database, install new one
/// in passed data directory, with passed
/// music directory (`path`).
///
/// @param `path`: Music directory argument.
/// @param `data_dir`: System data directory.
fn update_db(path: &Path, data_dir: &Path) -> Result<Connection> {
    // Make sure argument is valid.
    let music_dir = real::gatekeeper(path)
        .wrap_err_with(|| format!("Failed condition for: argument music directory `{path:?}`!"))?;

    println!(
        ":: {} {}…",
        "Creating new database for".green(),
        path.display().blue()
    );

    if data_dir.join("museum/music.db3").exists() {
        println!("==> {}…", "Deleting old database".purple());
        real::del_old_db(data_dir)?;
    }

    println!(":: {}…", "Searching for music".yellow());
    let files = real::find_music(&music_dir)
        .wrap_err_with(|| format!("Failed to find music files with `fd` from {music_dir:?}!"))?;
    // TODO: support multiple music file formats.
    println!("==> {} flac files found!", files.len().green().bold());

    println!(
        ":: {}… {}",
        "Starting to catalogue music in SQLite".yellow(),
        "This may take a while!".red().bold()
    );

    let db_conn = db::init(&files, data_dir).wrap_err("Failed to initialize SQLite database.")?;
    println!("==> {}", "Music catalogue complete!".green());

    Ok(db_conn)
}
