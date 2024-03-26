use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use color_eyre::eyre::{bail, Result, WrapErr};
use rusqlite::Connection;

// Add timestamp to touches/skips?
// This way later add decay system,
// so older touches and skips are removed?
//
// Would probably require an extra type just for interactions…
#[derive(Debug, Default)]
struct Song {
    /// SQL — limit on how many songs can be cataloged.
    #[allow(dead_code)]
    id: u32,
    // Change to PathBuf for stability?
    path: String,
    /// How often the song has been included in the que.
    /// $listens = touches -skips$
    /// $score   = listens - skips * aggression$
    touches: u32,
    /// When the user skips the song.
    skips: u32,
    /// Calculated score.
    score: Option<f64>,
}

impl Song {
    fn calc_score(&self) -> f64 {
        let listens = f64::from(self.touches - self.skips);
        let skips = f64::from(self.skips);
        let mut score: f64;

        // 30 seems good, as the difference
        // first gets doubled (5 -> 10),
        // and then 10 -> 15,
        // and finally doubled again (15 -> 30).
        if self.touches < 30 {
            let (weight_listens, weight_skips) = self.weight();
            score = weight_listens * listens - weight_skips * skips;
        } else {
            // Skips may be larger than listens.
            score = self.dampen() * listens - self.dampen() * skips;
        }

        if score < 0.0 {
            score = 0.0;
        }
        score
    }

    /// Weight calculation for songs with low touches (<30).
    /// Returns (`listens_weight`, `skips_weight`)
    ///
    /// Could, in theory, be used with values over 30,
    /// but this is not recommended — use [logarithmic
    /// dampening](<`fn dampen(touches)`>) instead.
    ///
    /// # `touches < 5`
    ///
    /// Listens are more important than skips
    /// This means that early, anecdotal skips are disregarded.
    ///
    /// # `touches <= 15`
    ///
    /// Listens are equally important to skips.
    ///
    /// # `touches > 15`
    ///
    /// Skips are more important than listens.
    /// this means skips still take an effect,
    /// and the algo learns with stability.
    ///
    fn weight(&self) -> (f64, f64) {
        // Need fine-tuning.
        let low = 0.5;
        let medium = 1.0;
        let high = 2.0;

        // These could also use some fine-tuning.
        // Currently using this *with* a logarithmic function
        // for the later stages. Thats why `big_threshold`
        // is so small.
        let small_threshold = 5;
        let big_threshold = 15;

        if self.touches < small_threshold {
            // Listens are more important than skips
            // This means that early, anecdotal skips are disregarded.
            (high, low)
        } else if self.touches <= big_threshold {
            // Listens are equally important to skips.
            (medium, medium)
        } else {
            // Skips are more important than listens.
            // So skips still take an effect,
            // and the algo learns with stability.
            (low, high)
        }
    }

    /// Logarithmic dampening function.
    /// Returns weight.
    ///
    /// Meant to be used for songs with over 30 touches.
    /// Very slow increase in weight, as touches incease,
    /// meaning that skips steadily have more importance.
    ///
    /// Causes recent preferences to rule king.
    ///
    fn dampen(&self) -> f64 {
        // `+1` just in case.
        // `1.2` seems to be ideal.
        f64::from(self.touches + 1).log(1.2)
    }
}

fn main() -> Result<()> {
    color_eyre::install().wrap_err("Failed to install error handling with `color-eyre`!")?;

    println!("WARNING: currently, museum has no memory. This will be implemented soon!");

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        bail!("Missing argument: `music_dir`!");
    }

    if &args[1] == "--help" || &args[1] == "-h" {
        bail!("Help has not been implemented yet. Please view the README.md!");
    }

    // TODO: support multiple music dirs.
    let music_dir = gatekeeper(&PathBuf::from(&args[1])).wrap_err_with(|| format!("Failed condition for: argument music directory `{}`!", args[1]))?;

    if music_dir.is_absolute() && music_dir.is_dir() && music_dir.exists() {
        println!(":: Searching for music…");
        // A little redundant, but more future proof.
        let files = map_path_to_song(
            &find_music(&music_dir).wrap_err_with(|| format!("Failed to find music files with `fd` from {music_dir:?}!"))?,
        );
        // TODO: support multiple music file formats.
        println!("==> {} flac files found!", files.len());

        println!(":: Starting to catalogue music in SQLite…");
        // TODO: persistent SQLite DB.
        let conn = init_db().wrap_err("Failed to initialize in-memory SQLite database.")?;
        // TODO: `update_db()`.
        insert_db(&files, &conn).wrap_err_with(|| format!("Failed to INSERT songs INTO database `{conn:?}`."))?;
        println!("==> Music catalogue complete!");

        println!(":: Displaying catalogued songs in database…");
        // TODO: `retrieve_song_obj()`.
        let songs = retrieve_songs_vec(&conn).wrap_err_with(|| format!("Failed to retrieve songs from `{conn:?}`."))?;
        for song in songs {
            println!("==> Found {}", song.path);
        }

        println!(":: THAT’S ALL, FOLKS!");
    }

    Ok(())
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
/// # Example
/// ```rust
/// let new_music_dir = gatekeeper(old_music_dir)?;
/// ```
fn gatekeeper(music_dir: &Path) -> Result<PathBuf> {
    if music_dir.is_relative() && music_dir.is_dir() && music_dir.exists() {
        println!(
            ":: Trying to convert `{}` into an absolute path…",
            music_dir.display()
        );
        let absolute_path = std::fs::canonicalize(music_dir)
            .wrap_err_with(|| format!("Failed to canonicalize relative music directory path: {music_dir:?}! Try using an absolute path."))?;
        println!("==> Converted into `{}`!", absolute_path.display());

        if music_dir.read_dir().wrap_err_with(|| format!("Failed to read inputed music directory: {music_dir:?}."))?.next().is_none() {
            bail!(
                "Music directory `{}` is empty — no music files to catalog.",
                music_dir.display()
            );
        }

        return Ok(absolute_path);
    } else if music_dir.is_file() {
        bail!(
            "Music directory `{}` is not a valid music *directory*.",
            music_dir.display()
        );
    }

    if music_dir.read_dir()?.next().is_none() {
        bail!(
            "Music directory `{}` is empty — no music files to catalog.",
            music_dir.display()
        );
    }

    Ok(music_dir.to_owned())
}

// Search `music_dir` for music files,
// and collect them in a vector.
fn find_music(music_dir: &Path) -> Result<Vec<PathBuf>> {
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

    let output = child.wait_with_output().wrap_err("Failed to collect `fd`s output!")?;

    let files: Vec<PathBuf> = output
        .stdout
        .lines()
        // Can’t figure out how *not* to use unwrap here.
        .map(|l| PathBuf::from(l.unwrap()))
        .collect();

    if files.is_empty() {
        bail!(
            "No music (.flac) files found in `{:?}`.",
            music_dir.display()
        );
    }

    Ok(files)
}

// Temp fn, to be replaced with long-term storage in SQLite.
fn map_path_to_song(paths: &[PathBuf]) -> Vec<Song> {
    paths
        .iter()
        .map(|path| Song {
            path: path.to_str().unwrap().to_string(),
            ..Default::default()
        })
        .collect()
}

// Starts (for now) in-memory SQLite database,
// and adds `song` table to it with error handling.
fn init_db() -> Result<Connection> {
    let conn = Connection::open_in_memory().wrap_err("Rusqlite in-memory connection refused.")?;

    conn.execute(
        "CREATE TABLE song (
            id      INTEGER PRIMARY KEY,
            path    TEXT NOT NULL,
            touches INTEGER NOT NULL,
            skips   INTEGER NOT NULL,
            score   BLOB
        )",
        (),
    )
    .wrap_err_with(|| format!("Invalid SQL command when CREATEing song TABLE in `{conn:?}`."))?;

    Ok(conn)
}

// Replace for `update_db` later.
fn insert_db(songs: &[Song], conn: &Connection) -> Result<()> {
    for song in songs {
        let score: Option<f64> = if song.score.is_none() {
            Some(song.calc_score())
        } else {
            song.score
        };

        conn.execute(
            "INSERT INTO song (path, touches, skips, score) VALUES (?1, ?2, ?3, ?4)",
            (&song.path, &song.touches, &song.skips, score),
        )
        .wrap_err_with(|| format!("Invalid SQL statement when INSERTing Song INTO database.\nSong: {song:?}.\nDB: {conn:?}."))?;
    }

    Ok(())
}

fn retrieve_songs_vec(conn: &Connection) -> Result<Vec<Song>> {
    let mut stmt = conn
        .prepare("SELECT * FROM song")
        .wrap_err_with(|| format!("Invalid SQL statement when SELECTing all FROM song in {conn:?}."))?;

    let song_iter = stmt
        .query_map([], |row| {
            Ok(Song {
                id: row.get(0)?,
                path: row.get(1)?,
                touches: row.get(2)?,
                skips: row.get(3)?,
                score: row.get(4)?,
            })
        })
        .wrap_err("Cannot query song.")?;

    let mut songs: Vec<Song> = Vec::new();
    for song in song_iter {
        // TODO: remove .unwrap().
        songs.push(song.wrap_err("Queried song unwrap failed.")?);
    }

    Ok(songs)
}
