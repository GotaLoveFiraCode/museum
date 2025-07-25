# Muse v2 API Documentation

This document provides comprehensive API documentation for all modules, functions, and data structures in Muse v2.

## 📋 Table of Contents

- [Data Structures](#data-structures)
- [Module: main](#module-main)
- [Module: cli](#module-cli)
- [Module: db](#module-db)
- [Module: algorithm](#module-algorithm)
- [Module: mpd_client](#module-mpd_client)
- [Module: queue](#module-queue)
- [Module: config](#module-config)
- [Error Handling](#error-handling)
- [Usage Examples](#usage-examples)

## 🗃️ Data Structures

### Song

Core data structure representing a song in the database.

```rust
pub struct Song {
    pub id: i64,           // Primary key in database
    pub path: String,      // Absolute file path
    pub artist: String,    // Artist name
    pub album: String,     // Album name
    pub title: String,     // Song title
    pub touches: u32,      // Number of times suggested by algorithm
    pub listens: u32,      // Number of times played completely
    pub skips: u32,        // Number of times skipped
    pub loved: bool,       // User-marked as loved
}
```

**Usage**:
```rust
let song = Song {
    id: 1,
    path: "/music/artist/album/song.flac".to_string(),
    artist: "The Beatles".to_string(),
    album: "Abbey Road".to_string(),
    title: "Come Together".to_string(),
    touches: 15,
    listens: 12,
    skips: 3,
    loved: true,
};
```

### SongConnection

Represents a connection between two songs (currently unused but in schema).

```rust
pub struct SongConnection {
    pub from_song_id: i64,  // Source song ID
    pub to_song_id: i64,    // Target song ID
    pub count: u32,         // Connection strength
}
```

### QueuedSong

Simplified song representation for queue operations.

```rust
#[derive(Clone)]
pub struct QueuedSong {
    pub path: String,      // File path for MPD
    pub artist: String,    // Artist for display
    pub title: String,     // Title for display
}
```

**Usage**:
```rust
let queued_song = QueuedSong {
    path: "artist/album/song.flac".to_string(),
    artist: "Miles Davis".to_string(),
    title: "Kind of Blue".to_string(),
};
```

## 🚀 Module: main

Application entry point and command orchestration.

### main() -> Result<()>

Main entry point that initializes logging and routes commands.

**Behavior**:
1. Initializes environment logger
2. Parses command-line arguments using Clap
3. Routes to appropriate module functions
4. Handles top-level error reporting

**Example**:
```bash
# Command routing examples
muse update /music        # -> db::update_database()
muse list                 # -> db::list_songs()
muse play algorithm       # -> mpd_client::play()
muse current "Song"       # -> queue::generate_current() + mpd_client::load_queue()
```

## 🖥️ Module: cli

Command-line interface definitions using Clap derive macros.

### Args

Main argument structure for the application.

```rust
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}
```

### Command

Enumeration of all available subcommands.

```rust
#[derive(Subcommand)]
pub enum Command {
    Update { path: PathBuf },          // Update database from directory
    List,                              // List all songs
    Play { mode: String },             // Play with mode (shuffle/algorithm)
    Current { song: String },          // Generate current queue
    Thread { song: String },           // Generate thread queue
    Stream { song: String },           // Generate stream queue
}
```

**Command Arguments**:

- `Update { path }`: Directory path to scan for music files
- `Play { mode }`: Mode string ("shuffle" or "algorithm", defaults to "algorithm")
- `Current/Thread/Stream { song }`: Song name for queue generation (fuzzy search)

## 🗄️ Module: db

Database operations and schema management using SQLite.

### get_connection() -> Result<Connection>

Establishes database connection and initializes schema if needed.

**Returns**: SQLite connection object
**Side Effects**: Creates database file and tables if they don't exist

**Schema Created**:
```sql
CREATE TABLE songs (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    artist TEXT NOT NULL,
    album TEXT NOT NULL,
    title TEXT NOT NULL,
    touches INTEGER DEFAULT 0,
    listens INTEGER DEFAULT 0,
    skips INTEGER DEFAULT 0,
    loved INTEGER DEFAULT 0
);

CREATE TABLE connections (
    from_song_id INTEGER,
    to_song_id INTEGER,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (from_song_id, to_song_id),
    FOREIGN KEY (from_song_id) REFERENCES songs(id),
    FOREIGN KEY (to_song_id) REFERENCES songs(id)
);
```

### update_database(music_dir: PathBuf) -> Result<()>

Scans directory tree and adds new music files to database.

**Parameters**:
- `music_dir`: Root directory to scan

**Behavior**:
1. Walks directory tree recursively
2. Identifies music files by extension
3. Extracts metadata (currently simplified filename parsing)
4. Inserts new songs, ignores duplicates
5. Reports number of songs added

**Supported Extensions**: .flac, .mp3, .ogg, .m4a, .wav

**Example**:
```rust
db::update_database(PathBuf::from("/home/user/Music"))?;
```

### list_songs() -> Result<()>

Displays all songs in database with statistics.

**Output Format**:
```
Artist - Album - Title (touches: N, listens: N, skips: N)
```

**Sorting**: Alphabetical by artist, then album, then title

### get_song_by_name(name: &str) -> Result<Song>

Searches for song by title or artist using fuzzy matching.

**Parameters**:
- `name`: Search string (partial matches allowed)

**Returns**: First matching song
**Search Strategy**: SQL LIKE with wildcards on both title and artist fields

**Example**:
```rust
let song = db::get_song_by_name("Beatles")?;  // Matches any Beatles song
let song = db::get_song_by_name("Come Together")?;  // Matches by title
```

### get_song_connections(song_id: i64) -> Result<Vec<(Song, u32)>>

Retrieves all songs connected to given song with connection counts.

**Parameters**:
- `song_id`: Source song database ID

**Returns**: Vector of (connected_song, connection_count) tuples
**Sorting**: Ordered by connection count (strongest first)

**Example**:
```rust
let connections = db::get_song_connections(42)?;
for (song, count) in connections {
    println!("{} -> {} ({}x)", song.title, count);
}
```

### update_song_stats(song_id: i64, touched: bool, listened: bool, skipped: bool) -> Result<()>

Updates song statistics based on user interaction.

**Parameters**:
- `song_id`: Target song ID
- `touched`: Increment touches counter
- `listened`: Increment listens counter  
- `skipped`: Increment skips counter

**Behavior**: Increments specified counters atomically

### update_connection(from_id: i64, to_id: i64) -> Result<()>

Records or strengthens connection between two songs.

**Parameters**:
- `from_id`: Source song ID
- `to_id`: Target song ID

**Behavior**: Inserts new connection or increments existing count

## 🧮 Module: algorithm

Implementation of scoring algorithms from the Muse v2 specification.

### calculate_score(song: &Song) -> f64

Main scoring function implementing the PDF algorithm.

**Algorithm**:
```rust
if song.touches < 30 {
    let (weight_listens, weight_skips) = weight(song.touches);
    score = (weight_listens * listens) - (weight_skips * skips)
} else {
    let dampening = dampen(song.touches);
    score = dampening * listens - dampening * skips
}

if score < 0.0 { score = 0.0; }
if song.loved { score *= 2.0; }
```

**Returns**: Non-negative score (higher = more likely to be played)

**Special Cases**:
- Negative scores clamped to 0.0
- Loved songs get 2x multiplier
- New songs (< 30 touches) favor exploration
- Established songs (≥ 30 touches) use stable weighting

### weight(touches: u32) -> (u8, u8)

Dynamic weighting function for different experience levels.

**Returns**: (listen_weight, skip_weight) tuple

**Weighting Strategy**:
- `touches < 5`: (4, 1) - Heavy bias toward positive experiences
- `5 ≤ touches ≤ 15`: (2, 2) - Balanced learning phase
- `touches > 15`: (1, 4) - Stable preferences, skips matter more

**Example**:
```rust
let (listen_weight, skip_weight) = weight(3);  // Returns (4, 1)
let (listen_weight, skip_weight) = weight(10); // Returns (2, 2)
let (listen_weight, skip_weight) = weight(20); // Returns (1, 4)
```

### dampen(touches: u32) -> f64

Logarithmic dampening for songs with many touches.

**Formula**: `log₁.₂(touches + 1)`

**Purpose**: Prevents score inflation for frequently suggested songs

**Example**:
```rust
let factor = dampen(30);  // ≈ 7.8
let factor = dampen(100); // ≈ 12.3
```

### apply_connection_weight(base_score: f64, connection_count: u32) -> f64

Applies connection bonuses to base song scores.

**Parameters**:
- `base_score`: Score from simple algorithm
- `connection_count`: Number of times this connection was observed

**Formula**: `base_score * log₁.₂(connection_count + 1)`

**Returns**: Enhanced score incorporating relationship strength

## 🎵 Module: mpd_client

MPD integration using mpc command-line tool.

### get_client() -> Result<()>

Verifies MPD and mpc availability.

**Behavior**: 
- Executes `mpc version` to test connectivity
- Returns error if MPD is not running or mpc is not installed

**Example**:
```rust
mpd_client::get_client()?;  // Throws error if MPD unavailable
```

### play(mode: String) -> Result<()>

Starts playback in specified mode.

**Parameters**:
- `mode`: "shuffle" or "algorithm"

**Shuffle Mode**:
1. Clears current queue
2. Enables random mode
3. Starts playback

**Algorithm Mode**:
1. Calculates scores for all songs
2. Loads top 50 songs into queue
3. Starts playback

### load_queue(queue: Vec<QueuedSong>) -> Result<()>

Loads generated queue into MPD.

**Parameters**:
- `queue`: Vector of songs to add

**Behavior**:
1. Clears current MPD queue
2. Adds each song using `mpc add`
3. Starts playback
4. Logs queue size

**Path Handling**: Strips leading "/" for MPD compatibility

### get_current_song() -> Result<Option<String>>

Retrieves currently playing song path.

**Returns**: File path of current song or None if nothing playing

**Implementation**: Uses `mpc current -f "%file%"`

### handle_song_finished(current_path: &str, next_path: &str) -> Result<()>

Records song transition for learning (future use).

**Parameters**:
- `current_path`: Song that just finished
- `next_path`: Song that started playing

**Behavior**:
1. Updates listen count for finished song
2. Updates touch count for new song
3. Records connection between songs

### handle_song_skipped(current_path: &str) -> Result<()>

Records song skip for learning (future use).

**Parameters**:
- `current_path`: Song that was skipped

**Behavior**: Increments skip count for specified song

## 🔀 Module: queue

Queue generation logic for three queue types.

### generate_current(song_name: &str) -> Result<Vec<QueuedSong>>

Generates "Current" queue with dual-path mixing.

**Parameters**:
- `song_name`: Starting song (fuzzy search)

**Algorithm**:
1. Find starting song by name
2. Get all connections from starting song
3. Select top 2 connections by score
4. Generate path from each connection
5. Interleave the two paths
6. Ensure 9-27 song length

**Returns**: Mixed queue with variety from two connection chains

**Example**:
```rust
let queue = queue::generate_current("Kind of Blue")?;
// Queue might contain: [Kind of Blue, Song A, Song X, Song B, Song Y, ...]
```

### generate_thread(song_name: &str) -> Result<Vec<QueuedSong>>

Generates "Thread" queue with single-path focus.

**Parameters**:
- `song_name`: Starting song (fuzzy search)

**Algorithm**:
1. Find starting song by name
2. Follow strongest connection chain
3. Continue until no positive-score connections
4. Ensure 9-27 song length

**Returns**: Focused queue following single musical thread

### generate_stream(song_name: &str) -> Result<Vec<QueuedSong>>

Generates "Stream" queue for algorithm training.

**Parameters**:
- `song_name`: Starting song (fuzzy search)

**Algorithm**:
1. Start with specified song
2. For each subsequent position:
   - If < 3 connections available, pick random song
   - Otherwise, randomly select from positive-score connections
3. Continue until exactly 30 songs

**Returns**: Training queue with exploration and randomness

### generate_path(start_id: i64, max_length: usize) -> Result<Vec<QueuedSong>>

Helper function to generate connection path.

**Parameters**:
- `start_id`: Starting song database ID
- `max_length`: Maximum path length

**Returns**: Path following strongest connections

### get_random_song() -> Result<Option<QueuedSong>>

Selects high-scoring random song for queue padding.

**Algorithm**:
1. Get 10 random songs from database
2. Calculate scores for each
3. Return highest-scoring song

**Returns**: Best song from random sample

## ⚙️ Module: config

Configuration and data directory management.

### get_db_path() -> Result<PathBuf>

Returns platform-appropriate database file path.

**Locations**:
- Linux: `~/.local/share/muse/music.db`
- macOS: `~/Library/Application Support/muse/music.db`
- Windows: `%APPDATA%\muse\music.db`

**Behavior**: Creates directory if it doesn't exist

**Example**:
```rust
let db_path = config::get_db_path()?;
println!("Database: {}", db_path.display());
```

## ❌ Error Handling

All functions return `anyhow::Result<T>` for consistent error handling.

### Common Error Types

**Database Errors**:
- File permissions
- SQLite constraint violations
- Disk space issues

**MPD Errors**:
- MPD not running
- mpc not installed
- Invalid song paths

**File System Errors**:
- Invalid music directory
- Permission denied
- Unsupported file formats

### Error Context

Functions provide descriptive error context:

```rust
db::update_database(path)
    .context("Failed to update music database")?;

mpd_client::get_client()
    .context("Failed to connect to MPD. Make sure MPD is running on localhost:6600")?;
```

## 💡 Usage Examples

### Basic Library Setup

```rust
use muse::{db, mpd_client};

// Initialize database
let music_dir = std::path::PathBuf::from("/home/user/Music");
db::update_database(music_dir)?;

// Verify MPD connection
mpd_client::get_client()?;

// Start algorithmic playback
mpd_client::play("algorithm".to_string())?;
```

### Queue Generation

```rust
use muse::{queue, mpd_client};

// Generate different queue types
let current_queue = queue::generate_current("Miles Davis")?;
let thread_queue = queue::generate_thread("Kind of Blue")?;
let stream_queue = queue::generate_stream("So What")?;

// Load any queue
mpd_client::load_queue(current_queue)?;
```

### Algorithm Scoring

```rust
use muse::{db, algorithm};

// Get song and calculate score
let song = db::get_song_by_name("Come Together")?;
let base_score = algorithm::calculate_score(&song);

// Apply connection weighting
let connections = db::get_song_connections(song.id)?;
for (connected_song, count) in connections {
    let connected_score = algorithm::calculate_score(&connected_song);
    let final_score = algorithm::apply_connection_weight(connected_score, count);
    println!("{}: {:.2}", connected_song.title, final_score);
}
```

### Custom Metadata Processing

```rust
use muse::db;

// Process song with custom metadata
let song = db::Song {
    id: 0,  // Will be set by database
    path: "/music/artist/song.flac".to_string(),
    artist: "Custom Artist".to_string(),
    album: "Custom Album".to_string(),
    title: "Custom Title".to_string(),
    touches: 0,
    listens: 0,
    skips: 0,
    loved: false,
};

// Calculate algorithm score
let score = algorithm::calculate_score(&song);
println!("New song score: {:.2}", score);
```

---

**This API documentation covers all public interfaces in Muse v2. For implementation details, see [DEVELOPMENT.md](DEVELOPMENT.md).**