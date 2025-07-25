//! Database operations for music library management.
//! 
//! Handles SQLite operations for song metadata, statistics, and connections.

use anyhow::{Result, Context};
use rusqlite::{Connection, params, OptionalExtension};
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use std::io::Write;
use crate::config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub id: i64,
    pub path: String,
    pub artist: String,
    pub album: String,
    pub title: String,
    pub touches: u32,
    pub listens: u32,
    pub skips: u32,
    pub loved: bool,
}

pub fn get_connection() -> Result<Connection> {
    let db_path = config::get_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {}", db_path.display()))?;
    
    // Initialize schema
    conn.execute(
        "CREATE TABLE IF NOT EXISTS songs (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            title TEXT NOT NULL,
            touches INTEGER DEFAULT 0,
            listens INTEGER DEFAULT 0,
            skips INTEGER DEFAULT 0,
            loved INTEGER DEFAULT 0
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY,
            from_song_id INTEGER NOT NULL,
            to_song_id INTEGER NOT NULL,
            count INTEGER DEFAULT 1,
            FOREIGN KEY (from_song_id) REFERENCES songs (id),
            FOREIGN KEY (to_song_id) REFERENCES songs (id),
            UNIQUE(from_song_id, to_song_id)
        )",
        [],
    )?;
    
    // Create indexes for performance
    conn.execute("CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_songs_album ON songs(album)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_songs_title ON songs(title)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_from ON connections(from_song_id)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_to ON connections(to_song_id)", [])?;

    Ok(conn)
}

pub fn init_database(path: &Path, force: bool, extract_metadata: bool) -> Result<()> {
    let start_time = Instant::now();
    let db_path = config::get_db_path()?;
    
    println!("🎵 Muse Database Initialization");
    println!("Music directory: {}", path.display());
    println!("Database: {}", db_path.display());
    println!("Extract metadata: {extract_metadata}");
    
    if force && db_path.exists() {
        println!("🗑️  Removing existing database...");
        std::fs::remove_file(&db_path)?;
    }
    
    let conn = get_connection()?;
    
    println!("📁 Scanning for music files...");
    let music_files = find_music_files(path)?;
    let total_files = music_files.len();
    println!("Found {total_files} music files");
    
    if total_files == 0 {
        println!("⚠️  No music files found in {}", path.display());
        return Ok(());
    }
    
    println!("💿 Processing files...");
    let mut processed = 0;
    let tx = conn.unchecked_transaction()?;
    
    for file_path in music_files {
        if let Some((artist, album, title)) = if extract_metadata {
            extract_metadata_from_file(&file_path)?
        } else {
            extract_metadata_from_path(&file_path)
        } {
            let path_str = file_path.to_string_lossy();
            
            tx.execute(
                "INSERT OR IGNORE INTO songs (path, artist, album, title) VALUES (?1, ?2, ?3, ?4)",
                params![path_str, artist, album, title],
            )?;
            
            processed += 1;
            if processed % 100 == 0 {
                print!("\r💿 Processed {processed}/{total_files} files");
                std::io::stdout().flush().unwrap();
            }
        }
    }
    
    tx.commit()?;
    
    let duration = start_time.elapsed();
    println!("\n✅ Database initialized successfully!");
    println!("📊 Processed {} songs in {:.2}s", processed, duration.as_secs_f64());
    
    Ok(())
}

pub fn update_database(path: &Path, scan_depth: u32, remove_missing: bool) -> Result<()> {
    let start_time = Instant::now();
    
    println!("🔄 Muse Database Update");
    println!("Music directory: {}", path.display());
    println!("Scan depth: {scan_depth} levels");
    println!("Remove missing: {remove_missing}");
    
    let conn = get_connection()?;
    
    // Get existing songs
    let mut existing_songs: HashMap<String, i64> = HashMap::new();
    let mut stmt = conn.prepare("SELECT id, path FROM songs")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(0)?))
    })?;
    
    for row in rows {
        let (path, id) = row?;
        existing_songs.insert(path, id);
    }
    
    println!("Found {} existing songs", existing_songs.len());
    
    // Scan for new files
    println!("📁 Scanning for music files...");
    let music_files = find_music_files_with_depth(path, scan_depth)?;
    let total_files = music_files.len();
    println!("Found {total_files} music files");
    
    let mut added = 0;
    let mut updated = 0;
    let mut removed = 0;
    
    // Track which files we've seen
    let mut seen_files: HashSet<String> = HashSet::new();
    
    println!("💿 Processing files...");
    let tx = conn.unchecked_transaction()?;
    
    for file_path in music_files {
        let path_str = file_path.to_string_lossy().to_string();
        seen_files.insert(path_str.clone());
        
        if existing_songs.contains_key(&path_str) {
            // File already exists, could update metadata here
            updated += 1;
        } else {
            // New file, add it
            if let Some((artist, album, title)) = extract_metadata_from_path(&file_path) {
                tx.execute(
                    "INSERT INTO songs (path, artist, album, title) VALUES (?1, ?2, ?3, ?4)",
                    params![path_str, artist, album, title],
                )?;
                added += 1;
            }
        }
        
        if (added + updated) % 100 == 0 {
            print!("\r💿 Processed {}/{} files", added + updated, total_files);
            std::io::stdout().flush().unwrap();
        }
    }
    
    // Remove missing files if requested
    if remove_missing {
        println!("\n🗑️  Removing missing files...");
        for (existing_path, song_id) in existing_songs {
            if !seen_files.contains(&existing_path) {
                tx.execute("DELETE FROM songs WHERE id = ?1", params![song_id])?;
                tx.execute("DELETE FROM connections WHERE from_song_id = ?1 OR to_song_id = ?1", params![song_id])?;
                removed += 1;
            }
        }
    }
    
    tx.commit()?;
    
    let duration = start_time.elapsed();
    println!("\n✅ Database updated successfully!");
    println!("📊 Added: {added}, Updated: {updated}, Removed: {removed}");
    println!("⏱️  Completed in {:.2}s", duration.as_secs_f64());
    
    Ok(())
}

pub fn list_songs() -> Result<()> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare("SELECT id, path, artist, album, title, touches, listens, skips, loved FROM songs ORDER BY artist, album, title")?;
    
    let songs = stmt.query_map([], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            title: row.get(4)?,
            touches: row.get(5)?,
            listens: row.get(6)?,
            skips: row.get(7)?,
            loved: row.get::<_, i32>(8)? != 0,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    
    println!("📚 Songs in database: {}", songs.len());
    for song in songs {
        let loved = if song.loved { "❤️" } else { "  " };
        println!("{} {} - {} - {} (T:{} L:{} S:{})", 
            loved, song.artist, song.album, song.title, 
            song.touches, song.listens, song.skips);
    }
    
    Ok(())
}

pub fn get_song_by_name(name: &str) -> Result<Song> {
    let conn = get_connection()?;
    
    // Strategy 1: Direct field matching
    let mut stmt = conn.prepare(
        "SELECT id, path, artist, album, title, touches, listens, skips, loved 
         FROM songs WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1 LIMIT 1"
    )?;
    
    let pattern = format!("%{name}%");
    if let Some(song) = stmt.query_row(params![pattern], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            title: row.get(4)?,
            touches: row.get(5)?,
            listens: row.get(6)?,
            skips: row.get(7)?,
            loved: row.get::<_, i32>(8)? != 0,
        })
    }).optional()? {
        return Ok(song);
    }
    
    // Strategy 2: Combined format parsing
    if name.contains(" - ") {
        let parts: Vec<&str> = name.splitn(2, " - ").collect();
        if parts.len() == 2 {
            let (part1, part2) = (parts[0].trim(), parts[1].trim());
            
            // Try artist - title
            let mut stmt = conn.prepare(
                "SELECT id, path, artist, album, title, touches, listens, skips, loved 
                 FROM songs WHERE artist LIKE ?1 AND title LIKE ?2 LIMIT 1"
            )?;
            
            let pattern1 = format!("%{part1}%");
            let pattern2 = format!("%{part2}%");
            if let Some(song) = stmt.query_row(params![pattern1, pattern2], |row| {
                Ok(Song {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    artist: row.get(2)?,
                    album: row.get(3)?,
                    title: row.get(4)?,
                    touches: row.get(5)?,
                    listens: row.get(6)?,
                    skips: row.get(7)?,
                    loved: row.get::<_, i32>(8)? != 0,
                })
            }).optional()? {
                return Ok(song);
            }
        }
    }
    
    // Strategy 3: Fuzzy word matching
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() >= 2 {
        let pattern1 = format!("%{}%", words[0]);
        let pattern2 = format!("%{}%", words[1]);
        
        let mut stmt = conn.prepare(
            "SELECT id, path, artist, album, title, touches, listens, skips, loved 
             FROM songs WHERE 
             (title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1) AND
             (title LIKE ?2 OR artist LIKE ?2 OR album LIKE ?2) LIMIT 1"
        )?;
        
        if let Some(song) = stmt.query_row(params![pattern1, pattern2], |row| {
            Ok(Song {
                id: row.get(0)?,
                path: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                title: row.get(4)?,
                touches: row.get(5)?,
                listens: row.get(6)?,
                skips: row.get(7)?,
                loved: row.get::<_, i32>(8)? != 0,
            })
        }).optional()? {
            return Ok(song);
        }
    }
    
    Err(anyhow::anyhow!("No song found matching: '{}'", name))
}

pub fn get_song_connections(song_id: i64) -> Result<Vec<(Song, u32)>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT s.id, s.path, s.artist, s.album, s.title, s.touches, s.listens, s.skips, s.loved, c.count
         FROM connections c
         JOIN songs s ON c.to_song_id = s.id
         WHERE c.from_song_id = ?1
         ORDER BY c.count DESC"
    )?;
    
    let connections = stmt.query_map(params![song_id], |row| {
        let song = Song {
            id: row.get(0)?,
            path: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            title: row.get(4)?,
            touches: row.get(5)?,
            listens: row.get(6)?,
            skips: row.get(7)?,
            loved: row.get::<_, i32>(8)? != 0,
        };
        let count: u32 = row.get(9)?;
        Ok((song, count))
    })?.collect::<Result<Vec<_>, _>>()?;
    
    Ok(connections)
}

pub fn update_song_stats(song_id: i64, touched: bool, listened: bool, skipped: bool) -> Result<()> {
    let conn = get_connection()?;
    
    let mut updates = Vec::new();
    if touched {
        updates.push("touches = touches + 1");
    }
    if listened {
        updates.push("listens = listens + 1");
    }
    if skipped {
        updates.push("skips = skips + 1");
    }
    
    if !updates.is_empty() {
        let update_clause = updates.join(", ");
        let query = format!("UPDATE songs SET {update_clause} WHERE id = ?1");
        conn.execute(&query, params![song_id])?;
    }
    
    Ok(())
}

pub fn update_connection(from_id: i64, to_id: i64) -> Result<()> {
    let conn = get_connection()?;
    
    conn.execute(
        "INSERT INTO connections (from_song_id, to_song_id, count) 
         VALUES (?1, ?2, 1)
         ON CONFLICT(from_song_id, to_song_id) 
         DO UPDATE SET count = count + 1",
        params![from_id, to_id],
    )?;
    
    Ok(())
}


pub fn get_all_songs_for_completion(db_path: &str) -> Result<Vec<Song>> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT id, path, artist, album, title, touches, listens, skips, loved FROM songs")?;
    
    let songs = stmt.query_map([], |row| {
        Ok(Song {
            id: row.get(0)?,
            path: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            title: row.get(4)?,
            touches: row.get(5)?,
            listens: row.get(6)?,
            skips: row.get(7)?,
            loved: row.get::<_, i32>(8)? != 0,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    
    Ok(songs)
}

// Helper functions

fn find_music_files(dir: &Path) -> Result<Vec<PathBuf>> {
    find_music_files_with_depth(dir, u32::MAX)
}

fn find_music_files_with_depth(dir: &Path, max_depth: u32) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let extensions = ["mp3", "flac", "ogg", "m4a", "wav", "opus"];
    
    fn scan_directory(
        dir: &Path, 
        files: &mut Vec<PathBuf>, 
        extensions: &[&str], 
        current_depth: u32, 
        max_depth: u32
    ) -> Result<()> {
        if current_depth > max_depth {
            return Ok(());
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                scan_directory(&path, files, extensions, current_depth + 1, max_depth)?;
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if extensions.contains(&ext_str.as_str()) {
                    files.push(path);
                }
            }
        }
        Ok(())
    }
    
    scan_directory(dir, &mut files, &extensions, 0, max_depth)?;
    Ok(files)
}

fn extract_metadata_from_path(path: &Path) -> Option<(String, String, String)> {
    let path_str = path.to_string_lossy();
    let parts: Vec<&str> = path_str.split('/').collect();
    
    if parts.len() >= 3 {
        let artist = parts[parts.len() - 3].to_string();
        let album = parts[parts.len() - 2].to_string();
        let filename = path.file_stem()?.to_string_lossy().to_string();
        Some((artist, album, filename))
    } else {
        let filename = path.file_stem()?.to_string_lossy().to_string();
        Some(("Unknown".to_string(), "Unknown".to_string(), filename))
    }
}

fn extract_metadata_from_file(path: &Path) -> Result<Option<(String, String, String)>> {
    // This is a placeholder - in a real implementation, we'd use a library like mp3-metadata
    // For now, fall back to path-based extraction
    Ok(extract_metadata_from_path(path))
}