use crate::song::Song;
use color_eyre::eyre::{ensure, Result, WrapErr};
use owo_colors::OwoColorize;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub mod command_handler;

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
pub fn gatekeeper(music_dir: &Path) -> Result<PathBuf> {
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
        format!(
            "Music directory `{}` does not exist!",
            music_dir.display()
        )
    );
    ensure!(
        music_dir.is_dir(),
        format!(
            "Argument `{}` is not a directory!",
            music_dir.display()
        )
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

/// Search `music_dir` for songs using `fd`,
/// and collect them in a vector.
///
/// Makes sure that files were found, otherwise
/// returns an error.
pub fn find_music(music_dir: &Path) -> Result<Vec<Song>> {
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

    let files: Vec<Song> = child
        .wait_with_output()
        .wrap_err("Failed to collect `fd`s output!")?
        .stdout
        .lines()
        // Alternatives to using `unwrap()`? Help wanted.
        .map(|path| Song {
            path: path.unwrap(),
            ..Default::default()
        })
        .collect();

    ensure!(
        !files.is_empty(),
        format!("No music (.flac) files found in `{music_dir:?}`.")
    );

    Ok(files)
}
