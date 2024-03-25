use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// # PLANNING
//
// ## Create struct for 'song' (file)
//
// include:
// - absolute path,
// - number of plays,
// - number of skips,
// - score?
//
// You could also just implement a function
// that calculates aggression and score.
//
// ### Search
//
// You should be able to search for specific file.
//
// - Just search through SQL?
//
// ## Algo
//
// Songs with under 10 listens (or more, not sure yet) get boosted to further
// develop database.
//
// Info gets stored in SQLite db.
//
// Number of listens - (skips * aggression) = score
//
// `aggression` starts out as 1, but increases
// as skips increase, so that skips really do something.
//
// A `listen` is when the user has listened to the whole song.
// I.e.
//  When a song comes up three times
//  and the user listens twice
//  and skips once
//  the final score is $2 - 1 = 1$
//
//  This means skips are literally counted as minus points ;P
//

use rusqlite::Connection;

// Add timestamp to touches/skips?
// This way later add decay system,
// so older touches and skips are removed?
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
    /// # Stuff I have enstablished
    ///
    /// Each song saves two stats: touches and skips
    ///
    /// `touches` are how often `museum` (the algo) has suggested the song.
    ///   `skips` are how often the user has   skipped the song.
    ///
    /// How often the user has actually listened to the whole song,
    /// can be calculated as such: $listens = touches - skips$
    ///
    /// # Things I want the algo to do
    ///
    /// 'boosted' means having a higher score.
    /// Songs are rated with they’re score.
    /// How do you calculate the score?
    ///
    /// Feel free to add more stats that should be stored with each song (variables).
    ///
    /// ## Early on
    ///
    /// Songs that have not been `touched`
    /// very often — say, less than five times —
    /// should be boosted, *even if* they have been
    /// `skipped` a few times (e.g. `listens` is very low).
    ///
    /// Example:
    ///
    /// Score should be generous
    /// $$touches = 3
    /// skips = 2
    /// score = ???$$
    ///
    /// This is so the algo has the chance to get feedback on all
    /// logged songs.
    /// I.e. songs that have only been touched a few times,
    /// are more likely to be suggested, so the algo can get an idea
    /// of how much the user likes said song.
    ///
    /// This means the algo doesn’t just end up *exclusively suggesting*
    /// the first 50 songs it suggest.
    ///
    /// Example:
    ///
    /// Score should be ca. equally generous.
    /// $$touches = 3
    /// skips = 0
    /// score = ???$$
    ///
    /// ## Middle stage
    ///
    /// When `touches` is still pretty low, `skips` shouldn’t take too much affect.
    /// More emphasis should be put on how often the user listens to the whole song.
    ///
    /// This way the user can skip a song a few times, without having to worry
    /// about never seeing it again (snowball effect).
    ///
    /// Example:
    ///
    /// Score should be fairly generous, as the song *has* been listened to 30 times.
    /// $$touches = 50
    /// skips = 20
    /// score = ???$
    ///
    /// Score should be very generous.
    /// $$touches = 50
    /// skips = 5
    /// score = ???$$
    ///
    /// Score should be strict.
    /// $$touches = 50
    /// skips = 45
    /// score = ???$$
    ///
    /// ## Late stage
    ///
    /// late-stage-songs: songs that have very high `touches`.
    ///
    /// These songs should take skips very seriously,
    /// so that if the user hasn’t enjoyed the song recently,
    /// the skips take noticable effect.
    ///
    /// Late stage songs should be downgraded (they’re score lowered)
    /// very aggressively. Not much heed should be taken the the `touches` stat.
    ///
    /// Example:
    ///
    /// Score should be harsh
    /// $$touches = 300
    /// skips = 130
    /// score = ???$$
    ///
    /// Score should be generous
    /// $$touches = 300
    /// skips = 40
    /// score = ???$$
    ///
    ///
    /// ## End result
    ///
    /// The end result is that songs with low `touches`,
    /// with medium `touches` and low `skips` (i.e. high `listens`),
    /// with medium `touches` and medium `skips`,
    /// and songs with high `touches` and low `skips` (i.e. high `listens`),
    /// are suggested aggressively.
    ///
    /// What are a few mathematical functions that matche all above data
    /// as closely as possible. How do you further prevent snowballing?
    ///
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
            score = self.dampen() * listens - self.dampen() * skips;
        }

        if score < 0.0 {
            score = 0.0;
        }
        score
    }

    /// Weight calculation.
    /// Returns (`listens_weight`, `skips_weight`)
    fn weight(&self) -> (f64, f64) {
        // Please make these better
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
            (high, low)
        } else if self.touches <= big_threshold {
            // Listens are equally important to skips.
            (medium, medium)
        } else {
            // Skips are more important than listens.
            (low, high)
        }
    }

    /// Logarithmic dampening function.
    /// Returns weight.
    fn dampen(&self) -> f64 {
        // `+1` just in case.
        // `1.2` seems to be ideal.
        f64::from(self.touches + 1).log(1.2)
    }
}

fn main() -> io::Result<()> {
    println!("WARNING: currently, museum has no memory! This will be implemented soon!");

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("ERROR: missing `music dir` argument.");
        println!("==>    `{} --help` for more info.", &args[0]);
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing argument: `music dir`",
        ));
    }

    if &args[1] == "--help" || &args[1] == "-h" {
        todo!("Help people.");
    } else {
        // TODO: support multiple music dirs.
        let music_dir = gatekeeper(&PathBuf::from(&args[1]))?;

        if music_dir.is_absolute() && music_dir.is_dir() && music_dir.exists() {
            println!(":: Searching for music…");
            // The only reason I initially save the path as PathBuf, is because
            // I am considering changing `Song.path` to the PathBuf type.
            let files = map_path_to_song(&find_music(&music_dir)?);
            println!("==> {} flac files found!", files.len());

            println!(":: Starting to catalogue music in SQLite…");
            let conn = init_db()?;
            insert_db(&files, &conn)?;
            println!("==> Music catalogue complete!");

            println!(":: Displaying catalogued songs in database…");
            let songs = retrieve_songs(&conn)?;
            for song in songs {
                println!("==> Found {song:?}");
            }

            println!(":: THAT’S ALL, FOLKS!");
        }
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
/// Returns `io::Result<PathBuf>` for convenience. (`gatekeeper(…)?`)
/// Might change in favour of `anyhow` crate.
///
/// Takes a reference and copies it — *optimization wanted*.
///
/// # Example
/// ```rust
/// let new_music_dir = gatekeeper(old_music_dir)?;
/// ```
fn gatekeeper(music_dir: &Path) -> io::Result<PathBuf> {
    if music_dir.is_relative() && music_dir.is_dir() && music_dir.exists() {
        println!(
            ":: Trying to convert `{}` into an absolute path…",
            music_dir.display()
        );
        let absolute_path = std::fs::canonicalize(music_dir)?;
        println!("==> Converted into `{}`!", absolute_path.display());

        if music_dir.read_dir()?.next().is_none() {
            println!(
                "ERROR: `{}` is empty — no music files to catalog.",
                music_dir.display()
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid music directory",
            ));
        }

        return Ok(absolute_path);
    } else if music_dir.is_file() || music_dir.is_symlink() {
        println!(
            "ERROR: `{}` is not a valid, *absolute* music *directory*.",
            music_dir.display()
        );
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid music directory",
        ));
        // A little redundant.
    }

    if music_dir.read_dir()?.next().is_none() {
        println!(
            "ERROR: `{}` is empty — no music files to catalog.",
            music_dir.display()
        );
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid music directory",
        ));
    }

    Ok(music_dir.to_owned())
}

// Search `music_dir` for music files,
// and collect them in a vector.
fn find_music(music_dir: &Path) -> io::Result<Vec<PathBuf>> {
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
        .expect("ERROR: failed to execute `fd` command.\nHINT:  try installing `fd[-find]` as a dependency.");

    let output = child.wait_with_output()?;

    let files: Vec<PathBuf> = output
        .stdout
        .lines()
        .map(|l| PathBuf::from(l.unwrap()))
        .collect();

    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "No music (.flac) files found in `{:?}`.",
                music_dir.display()
            ),
        ));
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
fn init_db() -> io::Result<Connection> {
    let conn = match Connection::open_in_memory() {
        Ok(i) => i,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Rusqlite in-memory connection error: {e}"),
            ));
        }
    };

    let conn_exct_rtrn = conn.execute(
        "CREATE TABLE song (
            id      INTEGER PRIMARY KEY,
            path    TEXT NOT NULL,
            touches INTEGER NOT NULL,
            skips   INTEGER NOT NULL,
            score   BLOB
        )",
        (),
    );

    if let Err(e) = conn_exct_rtrn {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid SQL command: {e}"),
        ));
    }

    Ok(conn)
}

// Replace for `update_db` later.
fn insert_db(songs: &[Song], conn: &Connection) -> io::Result<()> {
    for song in songs {
        let score: Option<f64> = if song.score.is_none() {
            Some(song.calc_score())
        } else {
            song.score
        };

        let conn_exct_rtrn = conn.execute(
            "INSERT INTO song (path, touches, skips, score) VALUES (?1, ?2, ?3, ?4)",
            (&song.path, &song.touches, &song.skips, score),
        );

        if let Err(e) = conn_exct_rtrn {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid SQL statement: {e}"),
            ));
        }
    }

    Ok(())
}

fn retrieve_songs(conn: &Connection) -> io::Result<Vec<Song>> {
    let mut stmt = match conn.prepare("SELECT * FROM song") {
        Ok(i) => i,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid SQL statement {e}."),
            ));
        }
    };

    let stmt_qry_map_rtrn = stmt.query_map([], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            touches: row.get(2)?,
            skips: row.get(3)?,
            score: row.get(4)?,
        })
    });

    let song_iter = match stmt_qry_map_rtrn {
        Ok(i) => i,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Cannot query SQL statement: {e}."),
            ));
        }
    };

    let mut songs: Vec<Song> = Vec::new();
    for song in song_iter {
        // TODO: remove .unwrap().
        songs.push(song.unwrap());
    }

    Ok(songs)
}
