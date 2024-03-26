use color_eyre::eyre::{ensure, Result, WrapErr};
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

    println!(":: Searching for music…");
    // A little redundant, but more future proof.
    let files = real::find_music(&music_dir).wrap_err_with(|| {
        format!("Failed to find music files with `fd` from {music_dir:?}!")
    })?;
    // TODO: support multiple music file formats.
    println!("==> {} flac files found!", files.len());

    println!(":: Starting to catalogue music in SQLite…");
    // TODO: persistent SQLite DB.
    let conn = db::init().wrap_err("Failed to initialize in-memory SQLite database.")?;
    // TODO: `update_db()`.
    db::insert(&files, &conn)
        .wrap_err_with(|| format!("Failed to INSERT songs INTO database `{conn:?}`."))?;
    println!("==> Music catalogue complete!");

    println!(":: Displaying catalogued songs in database…");
    // TODO: `retrieve_song_obj()`.
    let songs = db::retrieve_songs_vec(&conn)
        .wrap_err_with(|| format!("Failed to retrieve songs from `{conn:?}`."))?;
    for song in songs {
        println!("==> Found \"{}\"", song.path);
    }

    println!(":: THAT’S ALL, FOLKS!");

    Ok(())
}

