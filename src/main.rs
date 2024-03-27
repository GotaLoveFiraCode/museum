use color_eyre::eyre::{ensure, Result, WrapErr};
use etcetera::BaseStrategy;
use owo_colors::OwoColorize;
use std::path::PathBuf;

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

fn main() -> Result<()> {
    color_eyre::install().wrap_err("Failed to install error handling with `color-eyre`!")?;

    let args: Vec<String> = std::env::args().collect();

    ensure!(args.len() >= 2, "Missing argument: `music directory`!");
    ensure!(
        !(&args[1] == "--help" || &args[1] == "-h"),
        "Help has not been implemented yet. Please view the README.md!"
    );

    // TODO: support multiple music dirs.
    let music_dir = real::gatekeeper(&PathBuf::from(&args[1])).wrap_err_with(|| {
        format!(
            "Failed condition for: argument music directory `{}`!",
            args[1]
        )
    })?;

    println!(":: {}…", "Checking for existing music database".yellow());
    // Find system application data location.
    let data_dir = etcetera::choose_base_strategy()
        .wrap_err("Failed to set `etcetera`’s strategy.")?
        .data_dir();

    let conn: rusqlite::Connection;

    if data_dir.join("museum/music.db3").exists() {
        println!("==> {}", "Existing database found!".green());
        conn = db::connect(&data_dir)?;
    } else {
        println!(
            "==> {} {}…",
            "Existing database not found!".yellow(),
            "Creating new database".green()
        );

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
        // TODO: persistent SQLite DB.
        // TODO: `update_db()`.
        conn = db::init(&files, &data_dir).wrap_err("Failed to initialize SQLite database.")?;
        println!("==> {}", "Music catalogue complete!".green());
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
