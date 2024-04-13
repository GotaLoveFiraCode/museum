use crate::song::Song;
use color_eyre::eyre::{Result, WrapErr};
use rusqlite::Connection;
use std::path::Path;

/// Connect to DB. If DB doesn’t exist, create it. Always in same location, same name. Returns
/// `rusqlite::Connection`
pub fn connect(data_dir: &Path) -> Result<Connection> {
    let conn = Connection::open(data_dir.join("museum/music.db3"))
        .wrap_err_with(|| format!("Rusqlite DB connection refused. DB location: {data_dir:?}"))?;

    Ok(conn)
}

/// Starts `SQLite` database, and adds `song` table to it with error handling. Should only be
/// called once.
pub fn init(song: &[Song], data_dir: &Path) -> Result<Connection> {
    let mut conn = connect(data_dir).wrap_err("Connection refused when initializing DB.")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS song (
            id      INTEGER PRIMARY KEY,
            path    TEXT    NOT NULL,
            touches INTEGER NOT NULL,
            skips   INTEGER NOT NULL,
            score   BLOB
        )",
        (),
    )
    .wrap_err_with(|| format!("Invalid SQL command when CREATEing song TABLE in `{conn:?}`."))?;

    insert(song, &mut conn).wrap_err_with(|| {
        format!("Failed to INSERT songs INTO database `{conn:?}` while initializing.")
    })?;

    Ok(conn)
}

/// Only meant to be run once.
/// Part of initialization of DB.
/// Adds all songs to new database.
///
/// VERY FAST!
fn insert(songs: &[Song], conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;

    {
        let mut stmt =
            tx.prepare("INSERT INTO song (path, touches, skips, score) VALUES (?1, ?2, ?3, ?4)")?;

        for song in songs {
            stmt.execute((&song.path, &song.touches, &song.skips, &song.score))
                .wrap_err_with(|| {
                    format!(
                        "Invalid SQL statement when INSERTing Song INTO database!\nSong: {song:?}"
                    )
                })?;
        }
    }

    tx.commit().wrap_err("Commiting SQL transaction failed.")?;
    Ok(())
}

/// Iterate through `songs` and UPDATE each entry’s `touches` and `skips`
/// in the database with the same `id`.
///
/// Might switch to batch updates, or using `IN (…)`.
pub fn update_songs(songs: &[Song], conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;

    {
        let mut stmt =
            tx.prepare("UPDATE song SET touches = (?1), skips = (?2), score (?3) WHERE id = (?4)")?;

        for song in songs {
            // create identical temporary mutable song and calculate score.
            let mut temp_song = Song {
                id: song.id,
                path: song.path.clone(),
                touches: song.touches,
                skips: song.skips,
                ..Default::default()
            };
            temp_song.score = Some(song.calc_score());

            stmt.execute((temp_song.touches, temp_song.skips, temp_song.score, temp_song.id))
                .wrap_err_with(|| format!("Invalid SQL statement when UPDATEing song: {song:?}"))?;
        }
    }

    tx.commit().wrap_err("Commiting SQL transaction failed")?;
    Ok(())
}

/// Retrieves all songs from `SQLite` database,
/// and returns them as a vector of Songs (`Vec<Song>`),
/// wrapped in a Result.
pub fn retrieve_songs_vec(conn: &Connection) -> Result<Vec<Song>> {
    let mut stmt = conn.prepare("SELECT * FROM song").wrap_err_with(|| {
        format!("Invalid SQL statement when SELECTing all FROM song in {conn:?}.")
    })?;

    // Also retrieve `id`, to avoid duplicates later.
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
        .wrap_err("Cannot query songs.")?;

    let mut songs: Vec<Song> = Vec::new();
    // Could use extend, but then no error handling.
    for song in song_iter {
        songs.push(song.wrap_err("Queried song unwrap failed.")?);
    }

    Ok(songs)
}

/// Retrieve first `count` songs from this DB `conn`. Good for testing.
pub fn retrieve_first_songs(conn: &Connection, count: u8) -> Result<Vec<Song>> {
    let mut stmt = conn
        .prepare("SELECT * FROM song LIMIT (?1)")
        .wrap_err_with(|| {
            format!("Invalid SQL statement when SELECTing all FROM song in {conn:?}.")
        })?;

    // Also retrieve `id`, to avoid duplicates later.
    let song_iter = stmt
        .query_map([count], |row| {
            Ok(Song {
                id: row.get(0)?,
                path: row.get(1)?,
                touches: row.get(2)?,
                skips: row.get(3)?,
                score: row.get(4)?,
            })
        })
        .wrap_err("Cannot query songs.")?;

    let mut songs: Vec<Song> = Vec::new();
    // Could use extend, but then no error handling.
    for song in song_iter {
        songs.push(song.wrap_err("Queried song unwrap failed.")?);
    }

    Ok(songs)
}

pub fn retrieve_queue() {}
