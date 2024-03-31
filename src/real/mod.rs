use crate::song::Song;
use crate::db;

use color_eyre::eyre::{ensure, Result, WrapErr};
use owo_colors::OwoColorize;
use rusqlite::Connection;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub mod command_handler;

/// Delete old database, install new one
/// in passed data directory, with passed
/// music directory (`path`).
///
/// @param `path`: Music directory argument.
/// @param `data_dir`: System data directory.
pub fn update_db(path: &Path, data_dir: &Path) -> Result<Connection> {
    // Make sure argument is valid.
    let music_dir = gatekeeper(path)
        .wrap_err_with(|| format!("Failed condition for: argument music directory `{path:?}`!"))?;

    println!(
        ":: {} {}…",
        "Creating new database for".green(),
        path.display().blue()
    );

    if data_dir.join("museum/music.db3").exists() {
        println!("==> {}…", "Deleting old database".purple());
        del_old_db(data_dir)?;
    }

    println!(":: {}…", "Searching for music".yellow());
    let files = find_music(&music_dir)
        .wrap_err_with(|| format!("Failed to find music files with `fd` from {music_dir:?}!"))?;
    // TODO: support multiple music file formats.
    println!("==> {} flac files found!", files.len().green().bold());

    println!(
        ":: {}… {}",
        "Starting to catalogue music in SQLite".yellow(),
        "This may take a while!".red().bold()
    );

    let db_conn = db::init(&files, data_dir)
        .wrap_err("Failed to initialize SQLite database.")?;
    println!("==> {}", "Music catalogue complete!".green());

    Ok(db_conn)
}

/// Makes sure that the given `music_dir` is
///     a) a *absolute* path;
///     b) a *directory*;
///     c) *exists*;
///     d) not a *symlink*;
///     e) not *empty*.
///
/// If it is a *relative* path, `gatekeeper()` tries to convert it.
///
/// Takes a reference and copies it — *optimization wanted*.
///
///
///
/// # Example
/// ```rust
/// let new_music_dir = gatekeeper(old_music_dir)?;
/// ```
fn gatekeeper(music_dir: &Path) -> Result<PathBuf> {
    if music_dir.is_relative() {
        println!(
            ":: {} `{}` {}…",
            "Trying to convert".yellow(),
            music_dir.display().blue(),
            "into an absolute path".yellow(),
        );
        // Checks .exists() anyway. Directory gets checked later (.read_dir()).
        let absolute_path = std::fs::canonicalize(music_dir).wrap_err_with(|| {
            // Should I feel guilty?
            format!(
                "Failed to canonicalize relative music directory path: {:?}! {}",
                music_dir, "Try using an absolute path."
            )
        })?;
        println!("==> Converted into `{}`!", absolute_path.display().green());

        ensure!(
            music_dir
                .read_dir()
                .wrap_err_with(|| format!(
                    "Failed to read inputed music directory: {music_dir:?}."
                ))?
                .next()
                .is_some(),
            format!(
                "Music directory `{}` is empty — no music files to catalog.",
                music_dir.display()
            )
        );

        return Ok(absolute_path);
    }

    ensure!(
        music_dir.exists(),
        format!("Music directory `{}` does not exist!", music_dir.display())
    );
    ensure!(
        music_dir.is_dir(),
        format!("Argument `{}` is not a directory!", music_dir.display())
    );
    ensure!(
        music_dir.is_absolute(),
        format!(
            "Music directory `{}` is not absolute [INTERNAL ERROR]!",
            music_dir.display()
        )
    );

    ensure!(
        music_dir
            .read_dir()
            .wrap_err_with(|| format!("Failed to read inputed music directory: {music_dir:?}."))?
            .next()
            .is_some(),
        format!(
            "Music directory `{}` is empty — no music files to catalog.",
            music_dir.display()
        )
    );

    Ok(music_dir.to_owned())
}

/// Search `music_dir` for songs using `fd`, and collect them in a vector.
///
/// Makes sure that files were found, otherwise returns an error.
fn find_music(music_dir: &Path) -> Result<Vec<Song>> {
    // Create inner function for error handling and scoping. Kinda ugly.
    fn get_songs(child: std::process::Child) -> Result<Vec<Song>> {
        let binding = child
            .wait_with_output()
            .wrap_err("Failed to collect `fd`s output!")?;
        let lines = binding.stdout.lines();

        // Excellent example of pedantic error handling.
        let mut files: Vec<Song> = Vec::new();
        for path in lines {
            files.push(Song {
                path: path.wrap_err("Failed to unwrap path from `fd`")?,
                ..Default::default()
            });
        }

        Ok(files)
    }

    // `$ man fd`
    let child = Command::new("fd")
        // Allow for custom choice of file types. Settings files?
        .arg("-e")
        .arg("flac")
        // Add `-x {&args[0]} add_dir`
        .arg("-t")
        .arg("f")
        .arg(".")
        .arg(music_dir.to_str().unwrap())
        .stdout(Stdio::piped())
        .spawn()
        .wrap_err("Failed to spawn `fd`! Try installing the `fd-find` dependency.")?;

    let files = get_songs(child)?;

    ensure!(
        !files.is_empty(),
        format!("No music (.flac) files found in `{music_dir:?}`.")
    );

    Ok(files)
}

/// Delete existing database with `rm` system command.
///
/// @param `data_dir`: System data directory, passed to avoid re-computation.
fn del_old_db(data_dir: &Path) -> Result<()> {
    std::fs::remove_file(data_dir.join("museum/music.db3")).wrap_err_with(|| {
        format!(
            "Failed to delete existing database. Try running `rm \"{}/museum/music.db3\"`",
            data_dir.display()
        )
    })?;
    Ok(())
}
