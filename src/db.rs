use std::path::PathBuf;
use crate::song::Song;
use color_eyre::eyre::{Result, WrapErr};
use rusqlite::Connection;

/// Connect to DB.
/// If DB doesn’t exist, create it.
/// Always in same location, same name.
/// Returns `rusqlite::Connection`
pub fn connect(data_dir: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(data_dir.join("museum/music.db3"))
        .wrap_err_with(|| format!("Rusqlite DB connection refused. DB location: {data_dir:?}"))?;

    Ok(conn)
}

/// Starts `SQLite` database,
/// and adds `song` table to it with error handling.
/// Should only be called once.
pub fn init(song: &[Song], data_dir: &PathBuf) -> Result<Connection> {
    let conn = connect(data_dir).wrap_err("Connection refused when initializing DB.")?;

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

    insert(song, &conn).wrap_err_with(||
        format!("Failed to INSERT songs INTO database `{conn:?}` while initializing.")
    )?;

    Ok(conn)
}

/// Only meant to be run once.
/// Part of initialization of DB.
/// Adds all songs to new database.
fn insert(songs: &[Song], conn: &Connection) -> Result<()> {
    for song in songs {
        // Only bother calculating `score` when song gets `touched`…
        // let score: Option<f64> = if song.score.is_none() {
        //     Some(song.calc_score())
        // } else {
        //     song.score
        // };

        // How do I do this concurrently? HOW? IS IT EVEN POSSIBLE? HOW?!

        conn.execute(
            "INSERT INTO song (path, touches, skips, score) VALUES (?1, ?2, ?3, ?4)",
            (&song.path, &song.touches, &song.skips, &song.score),
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
        .wrap_err("Cannot query songs.")?;

    let mut songs: Vec<Song> = Vec::new();
    for song in song_iter {
        // TODO: remove .unwrap().
        songs.push(song.wrap_err("Queried song unwrap failed.")?);
    }

    Ok(songs)
}
