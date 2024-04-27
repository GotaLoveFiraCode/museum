use crate::song::Song;
use color_eyre::eyre::{Result, WrapErr};
use log::trace;
use rand::Rng;
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
            loved   INTEGER NOT NULL,
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
            tx.prepare("INSERT INTO song (path, touches, skips, loved, score) VALUES (?1, ?2, ?3, ?4, ?5)")?;

        for song in songs {
            let loved = match &song.loved {
                crate::song::Love::False => false,
                crate::song::Love::True => true,
            };

            stmt.execute((&song.path, &song.touches, &song.skips, loved, &song.score))
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
        let mut stmt = tx.prepare(
            "UPDATE song SET touches = (?1), skips = (?2), loved = (?3), score = (?4) WHERE id = (?5)",
        )?;

        for song in songs {
            // create identical temporary mutable song and calculate score.
            let mut temp_song = Song {
                id: song.id,
                path: song.path.clone(),
                touches: song.touches,
                skips: song.skips,
                loved: song.loved.clone(),
                ..Default::default()
            };
            temp_song.score = Some(song.calc_score());


            let loved = match &song.loved {
                crate::song::Love::False => false,
                crate::song::Love::True => true,
            };
            stmt.execute((
                temp_song.touches,
                temp_song.skips,
                loved,
                temp_song.score,
                temp_song.id,
            ))
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
                loved: match row.get(4).unwrap() {
                    1 => crate::song::Love::True,
                    _ => crate::song::Love::False,
                },
                score: row.get(5)?,
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
                loved: match row.get(4).unwrap() {
                    1 => crate::song::Love::True,
                    _ => crate::song::Love::False,
                },
                score: row.get(5)?,
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

/// A queue is always 15 songs.
/// Retrieves a set of somewhat random songs,
/// from the DB connection supplied (`conn`).
pub fn retrieve_rnd_queue(conn: &Connection) -> Result<Vec<Song>> {
    let mut stmt = conn
        .prepare("SELECT COUNT(id) FROM song")
        .wrap_err("Could not count database entries.")?;

    let db_rows = stmt
        .query_map([], |row| -> Result<u32, rusqlite::Error> { row.get(0) })?
        .next()
        .unwrap()?;
    trace!("{db_rows} entries in databas!");

    let mut queue: Vec<Song> = Vec::new();
    while queue.len() < 15 {
        // Ensure that the first, 5th, 10th, and 15th songs are more likely to
        // be known (already have a `score` value).
        if queue.is_empty() || queue.len() == 4 || queue.len() == 9 || queue.len() == 14 {
            let cmp_songs = [
                get_song_with_score(conn, db_rows)?,
                get_song_with_score(conn, db_rows)?,
                get_song_with_score(conn, db_rows)?,
                get_song_with_score(conn, db_rows)?
            ];
            let chosen_song = compare_and_choose_some(&cmp_songs);
            queue.push(chosen_song);
        } else {
            let cmp_songs = [
                get_random_song(conn, db_rows)?,
                get_random_song(conn, db_rows)?,
                get_random_song(conn, db_rows)?,
                get_random_song(conn, db_rows)?
            ];
            let chosen_song = compare_and_choose(&cmp_songs);
            queue.push(chosen_song);
        }
    }

    Ok(queue)
}

/// Chooses a random song with an already computed `score`.
/// If no song can be found, uses `get_random_song()`.
fn get_song_with_score(conn: &Connection, rows: u32) -> Result<Song> {
    let mut rng = rand::thread_rng();

    let mut stmt = conn.prepare("SELECT COUNT(*) FROM song WHERE score IS NOT NULL")?;
    let count: u32 = match stmt.query_row([], |row| row.get(0)) {
        Ok(num) => num,
        Err(rusqlite::Error::QueryReturnedNoRows) => return get_random_song(conn, rows),
        Err(err) => color_eyre::eyre::bail!(err),
    };

    // If no songs with `score` exist, just retrieve a random song.
    if count == 0 {
        trace!("Getting random song, as no song with `score` exists.");
        return get_random_song(conn, rows);
    }

    let rnd_offset = rng.gen_range(0..count);
    let mut stmt = conn.prepare("SELECT * FROM song WHERE score IS NOT NULL LIMIT 1 OFFSET ?")?;
    stmt.query_row([rnd_offset], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            touches: row.get(2)?,
            skips: row.get(3)?,
            loved: match row.get(4).unwrap() {
                1 => crate::song::Love::True,
                _ => crate::song::Love::False,
            },
            score: row.get(5)?,
        })
    })
        .wrap_err("Failed to query score song.")
}

/// Retrieve a random song.
fn get_random_song(conn: &Connection, rows: u32) -> Result<Song> {
    let song_id = get_rnd_row(rows);
    retrieve_song_by_id(conn, song_id)
}

/// New songs (None) are favoured.
/// Choose one song from given songs based on score.
fn compare_and_choose(songs: &[Song]) -> Song {
    songs.iter().max_by(|a, b| {
        match (a.score, b.score) {
            (Some(a), Some(b)) if a > b => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) if a < b => std::cmp::Ordering::Less,
            (Some(_), Some(_)) | (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
        }
    }).cloned().unwrap()
}

/// Same as `compare_and_choose`, but favoures Some-score songs.
fn compare_and_choose_some(songs: &[Song]) -> Song {
    songs.iter().max_by(|a, b| {
        match (a.score, b.score) {
            (Some(a), Some(b)) if a > b => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) if a < b => std::cmp::Ordering::Less,
            (Some(_), Some(_)) | (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
        }
    }).cloned().unwrap()
}

/// Generate random number between 1 and `rows` (inclusive).
/// Qualified.
fn get_rnd_row(rows: u32) -> u32 {
    rand::Rng::gen_range(&mut rand::thread_rng(), 1..=rows)
}

/// Find specific song by its id.
fn retrieve_song_by_id(conn: &Connection, id: u32) -> Result<Song> {
    let mut stmt = conn
        .prepare("SELECT * FROM song WHERE id = (?1)")
        .wrap_err_with(|| {
            format!("Invalid SQL statement when SELECTing all FROM song in {conn:?}.")
        })?;

    stmt.query_row([id], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            touches: row.get(2)?,
            skips: row.get(3)?,
            loved: match row.get(4).unwrap() {
                1 => crate::song::Love::True,
                _ => crate::song::Love::False,
            },
            score: row.get(5)?,
        })
    })
    .wrap_err("Failed to query song.")
}
