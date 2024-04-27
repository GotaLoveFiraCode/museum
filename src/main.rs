use color_eyre::eyre::{ensure, Result, WrapErr};
use etcetera::BaseStrategy;
use log::{info, warn};
use log4rs::config::Deserializers;
use owo_colors::OwoColorize;
use rusqlite::Connection;

/// Code related to Songs/algo specifically.
mod song;

/// Code related to real stuff.
/// This means:
/// - handling command-line arguments,
/// - finding files,
/// - converting files into non-real types (`Song`),
/// - boilerplate,
/// - etc.
mod real;

/// Play music.
mod playback;

/// Interact with the `SQLite` database.
mod db;

use clap::Parser;
use real::command_handler::Cli;

// All output-related functions are in `main`. All helper functions that are not strictly related
// to the algorithm or database are in `real`. Are algorithm functions et al. are in `song`, and
// all database functions et al are in `db`.

fn main() -> Result<()> {
    log4rs::init_file("./log4rs.yaml", Deserializers::default()).unwrap();
    color_eyre::install().wrap_err("Failed to install error handling with `color-eyre`!")?;

    // Arguments.
    let cli = Cli::parse();

    // Find system application data location.
    let data_dir = etcetera::choose_base_strategy()
        .wrap_err("Failed to set `etcetera`’s strategy.")?
        .data_dir();

    // Un-initialized connection to DB.
    let mut conn: Connection;

    // If user gave new music_dir:
    if let Some(path) = cli.update {
        conn = real::update_db(&path, &data_dir)
            .wrap_err_with(|| format!("Failed to update DB for {}!", path.display()))?;
    } else {
        info!("{}…", "Checking for existing music database".yellow());
        ensure!(
            data_dir.join("museum/music.db3").exists(),
            "No previous database found! Run `museum --help`."
        );
        info!(
            "{} {}",
            "Existing database found!".green(),
            "Use `-l` to list songs".italic()
        );

        conn = db::connect(&data_dir)?;
    }

    if cli.list {
        info!("{}…", "Displaying catalogued songs in database".yellow());
        // TODO: `retrieve_song_obj()`.
        let songs = db::retrieve_songs_vec(&conn)
            .wrap_err_with(|| format!("Failed to retrieve songs from `{conn:?}`."))?;
        for song in songs {
            // println!("==> Found \"{}\"", song.path.blue());
            println!("{}", song.path.blue());
        }
    }

    if cli.test_audio {
        info!("{}…", "Fetching 3 songs from DB to test play".yellow());
        let queue = db::retrieve_first_songs(&conn, 3)?;
        info!("{}…", "Playing audio".yellow());
        let new = playback::play_queue_with_cmds(&queue).wrap_err("Failed to play audio.")?;
        warn!("Didn’t update songs: {:?}", new.blue());
    }

    if cli.play_rnd {
        info!("Fetching random songs from DB to play…");
        let queue = db::retrieve_rnd_queue(&conn)?;
        info!("Successfully created queue!");

        info!("Playing audio…");
        let updated_queue = playback::play_queue_with_gui(&queue).unwrap();

        info!("Updating database…");
        db::update_songs(&updated_queue, &mut conn)?;
        info!("Successfully updated DB!");
    }

    info!("{}", "THAT’S ALL, FOLKS!".green().bold());
    Ok(())
}
