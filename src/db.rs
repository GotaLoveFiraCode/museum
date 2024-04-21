use crate::song::Song;
use color_eyre::eyre::{Result, WrapErr};
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
        let mut stmt = tx.prepare(
            "UPDATE song SET touches = (?1), skips = (?2), score = (?3) WHERE id = (?4)",
        )?;

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

            stmt.execute((
                temp_song.touches,
                temp_song.skips,
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

/// A queue is always 15 songs.
/// Retrieves a set of somewhat random songs,
/// from the DB connection supplied (`conn`).
pub fn retrieve_rnd_queue(conn: &Connection) -> Result<Vec<Song>> {
    let mut stmt = conn
        .prepare("SELECT COUNT(id) FROM song")
        .wrap_err("Could not count database entries.")?;

    // wtf
    let db_rows = stmt
        .query_map([], |row| -> Result<u32, rusqlite::Error> { row.get(0) })?
        .next()
        .unwrap()?;

    let mut queue: Vec<Song> = Vec::new();

    // TODO: This is really shitty code, change later.
    // Fills vec with songs (15 of them).
    while queue.len() < 15 {
        // Ensure that the first, 5th, 10th, and 15th songs are more likely to
        // be known (already have a `score` value).
        if queue.is_empty() || queue.len() == 4 || queue.len() == 9 || queue.len() == 14 {
            let song1 = get_song_with_score(conn, db_rows)?;
            let song2 = get_song_with_score(conn, db_rows)?;
            let chosen_song = compare_and_choose_some(song1, song2);
            queue.push(chosen_song);
        } else {
            let song1 = get_random_song(conn, db_rows)?;
            let song2 = get_random_song(conn, db_rows)?;
            let chosen_song = compare_and_choose(song1, song2);
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
            score: row.get(4)?,
        })
    })
    .wrap_err("Failed to query score song.")
}

/// Retrieve a random song.
fn get_random_song(conn: &Connection, rows: u32) -> Result<Song> {
    let song_id = get_rnd_row(rows);
    retrieve_song_by_id(conn, song_id)
}

/// New songs are favoured.
/// Choose between two songs based on score.
fn compare_and_choose(song1: Song, song2: Song) -> Song {
    match (song1.score, song2.score) {
        (Some(score1), Some(score2)) => {
            if score1 > score2 {
                song1
            } else {
                song2
            }
        }
        (Some(_), None) => song2,
        (None, Some(_) | None) => song1,
    }
}

/// Same as `compare_and_choose`, but favoures Some-score songs.
fn compare_and_choose_some(song1: Song, song2: Song) -> Song {
    match (song1.score, song2.score) {
        (Some(score1), Some(score2)) => {
            if score1 > score2 {
                song1
            } else {
                song2
            }
        }
        (None, Some(_)) => song2,
        (Some(_) | None, None) => song1,
    }
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
            score: row.get(4)?,
        })
    })
    .wrap_err("Failed to query song.")
}
