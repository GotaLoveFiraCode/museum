use color_eyre::eyre::{Result, WrapErr};
use rusqlite::Connection;
use crate::song::Song;

/// Starts (for now) in-memory `SQLite` database,
/// and adds `song` table to it with error handling.
pub fn init() -> Result<Connection> {
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

/// Replace for `update_db` later.
pub fn insert(songs: &[Song], conn: &Connection) -> Result<()> {
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

/// Retrieves all songs from `SQLite` database,
/// and returns them as a vector of Songs (`Vec<Song>`),
/// wrapped in a Result.
pub fn retrieve_songs_vec(conn: &Connection) -> Result<Vec<Song>> {
    let mut stmt = conn.prepare("SELECT * FROM song").wrap_err_with(|| {
        format!("Invalid SQL statement when SELECTing all FROM song in {conn:?}.")
    })?;

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

