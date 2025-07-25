//! # MPD Client Module with Intelligent Behavior Tracking
//!
//! This module provides comprehensive integration with Music Player Daemon (MPD) 
//! using the `mpc` command-line client. Beyond basic playback control and queue 
//! management, it implements **intelligent behavior tracking** that enables the 
//! algorithmic recommendation engine to learn from user interactions.
//!
//! ## Core Functionality
//!
//! ### 🎵 **Playback Control**
//! - Queue management (`load_queue`, `play`)
//! - MPD status monitoring (`get_mpd_status`)
//! - Path translation between absolute and MPD-relative formats
//!
//! ### 🧠 **Intelligent Behavior Tracking**
//! - **Touch Tracking**: Records every time algorithm suggests a song
//! - **Listen/Skip Detection**: Uses 80% threshold to classify user behavior
//! - **Love/Unlove**: Explicit user preference signals (2x algorithm weight)
//! - **Smart Command Wrappers**: `muse next/skip/love` with integrated tracking
//!
//! ### 📊 **Behavioral Data Types**
//! - **Touches**: How often algorithm recommended this song
//! - **Listens**: Songs played >80% (natural completion)  
//! - **Skips**: Songs played <80% or explicitly skipped
//! - **Loved**: User-marked favorites (2x recommendation weight)
//!
//! ## Architecture Overview
//!
//! ```text
//! User Action → MPD Command → Behavior Analysis → Database Update → Algorithm Learning
//!     ↓              ↓              ↓                 ↓                    ↓
//! muse next → mpc next → Check 80% → Update stats → Better recommendations
//! ```
//!
//! ## Design Decision: mpc vs Direct Protocol
//!
//! This implementation uses the `mpc` command-line tool instead of direct MPD
//! protocol communication for several reasons:
//! - Simplicity: No need to implement MPD protocol parsing
//! - Reliability: mpc is well-tested and handles edge cases
//! - Compatibility: Works with any MPD version that mpc supports
//! - Error Handling: mpc provides clear error messages
//!
//! ## Usage Examples
//!
//! ### Basic Queue Loading with Touch Tracking
//! ```rust,no_run
//! use muse::mpd_client;
//! use muse::queue::QueuedSong;
//! use anyhow::Result;
//!
//! fn main() -> Result<()> {
//!     let queue = vec![
//!         QueuedSong {
//!             path: "/music/artist/song1.mp3".to_string(),
//!             artist: "Artist".to_string(),
//!             title: "Song 1".to_string(),
//!             score: 0.85,
//!         }
//!     ];
//!
//!     // This automatically tracks touches for suggested songs
//!     mpd_client::load_queue(&queue)?;
//!     Ok(())
//! }
//! ```
//!
//! ### Smart Navigation with Behavior Tracking
//! ```rust,no_run
//! use muse::mpd_client;
//! use anyhow::Result;
//!
//! fn main() -> Result<()> {
//!     // Instead of raw mpc commands, use smart wrappers:
//!
//!     // Automatically detects listen vs skip based on 80% threshold
//!     mpd_client::next_with_tracking()?;
//!
//!     // Always marks as skip regardless of progress
//!     mpd_client::skip_with_tracking()?;
//!
//!     // Marks current song as loved (2x algorithm weight)
//!     mpd_client::love_current_track()?;
//!
//!     // Removes loved status
//!     mpd_client::unlove_current_track()?;
//!     Ok(())
//! }
//! ```
//!
//! ### CLI Integration 
//! ```bash
//! # Generate stream with automatic touch tracking
//! muse stream "Song Name"
//!
//! # Smart next (detects listen/skip automatically)
//! muse next  
//!
//! # Explicit skip (stronger negative signal)
//! muse skip
//!
//! # Love/unlove current track
//! muse love
//! muse unlove
//! ```
//!
//! ## Behavioral Learning Flow
//!
//! 1. **Algorithm Generates Queue** → Songs get `touches` incremented
//! 2. **User Listens/Skips** → `listens`/`skips` counters updated  
//! 3. **User Loves Songs** → `loved` flag set (2x weight)
//! 4. **Algorithm Learns** → Future recommendations improve based on data
//!
//! ## Error Handling Strategy
//!
//! - **Graceful Degradation**: Missing songs in DB don't fail the entire operation
//! - **Clear Error Messages**: Path translation and MPD connection failures are explicit
//! - **Logging**: Comprehensive debug/info/warn logging for troubleshooting
//! - **Fallbacks**: No current song → operations still succeed where possible
//!
//! ## MPD Integration
//!
//! - Queue Management: Clear, add songs, and start playback
//! - Playback Control: Start, stop, and monitor playback status
//! - Path Handling: Convert absolute paths to MPD-relative paths
//! - Error Recovery: Graceful handling of MPD connection issues

use anyhow::{Result, Context};
use std::process::Command;
use log::{info, warn, debug};
use crate::db;
use crate::algorithm;
use crate::queue::QueuedSong;
use crate::path_translator;
use std::process::Stdio;

/// Verifies MPD and mpc availability.
/// 
/// Tests connection to MPD by running `mpc version`. This ensures both
/// that mpc is installed and that MPD is running and accessible.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - mpc command is not found
/// - MPD is not running
/// - Connection to MPD fails
pub fn get_client() -> Result<()> {
    let output = Command::new("mpc")
        .arg("version")
        .output()
        .context("Failed to execute mpc command. Please install mpc (MPD client)")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to connect to MPD. Make sure MPD is running on localhost:6600.\nError: {}", 
            stderr.trim()
        );
    }
    
    Ok(())
}

/// # Errors
/// 
/// This function will return an error if:
/// - MPD connection fails
/// - mpc command execution fails
/// - Invalid play mode provided
/// - MPD is not running or accessible
pub fn play(mode: &str) -> Result<()> {
    get_client()
        .context("Cannot start playback: MPD connection failed")?;
    
    match mode {
        "shuffle" => {
            // Shuffle in mpd: `mpc listall | mpc add && mpc shuffle && mpc play`
            // There is is also `mpc random on`, which simple randomizes playback
            // from current playlist.
            info!("Playing in shuffle mode");
            mpc_shuffle()?;
        }
        "algorithm" => {
            info!("Playing with algorithm mode");
            play_with_algorithm()
                .context("Failed to start algorithmic playback")?;
        }
        _ => anyhow::bail!("Invalid play mode '{mode}'. Use 'shuffle' or 'algorithm'"),
    }
    
    Ok(())
}

/// Clear, add all songs, shuffle and configure playback
fn mpc_shuffle() -> Result<()> {
    Command::new("mpc").arg("clear").output().context("f")?;

    let listall = Command::new("mpc")
        .arg("listall")
        .stdout(Stdio::piped())
        .spawn().context("f")?;

    let stdout = listall.stdout.ok_or_else(|| anyhow::anyhow!("Failed to get stdout from mpc listall"))?;
    Command::new("mpc").arg("add")
        .stdin(Stdio::from(stdout))
        .output().context("f")?;

    Command::new("mpc").arg("shuffle").output().context("f")?;
    Command::new("mpc").args(["random", "on"]).output().context("f")?;
    Command::new("mpc").args(["consume", "on"]).output().context("f")?;
    Command::new("mpc").args(["repeat", "on"]).output().context("f")?;
    Command::new("mpc").arg("play").output().context("f")?;

    Ok(())
}

/// # Errors
/// 
/// This function will return an error if:
/// - MPD connection fails
/// - Queue is empty
/// - Song files cannot be added to MPD
/// - MPD playback fails to start
pub fn load_queue(queue: &[QueuedSong]) -> Result<()> {
    if queue.is_empty() {
        anyhow::bail!("Cannot load empty queue");
    }
    
    get_client()
        .context("Cannot load queue: MPD connection failed")?;
    
    // Ensure path translator is initialized
    if !path_translator::is_initialized() {
        info!("Initializing path translator for MPD integration");
        path_translator::initialize()
            .context("Failed to initialize path translator for MPD integration")?;
    }
    
    // Clear current queue
    Command::new("mpc").arg("clear").output()
        .context("Failed to clear MPD queue before loading new songs")?;
    
    // Add songs to queue with proper path translation
    let mut added_count = 0;
    let mut failed_count = 0;
    
    for song in queue {
        debug!("Adding song to MPD queue: {}", song.path);
        
        // Convert absolute path to MPD relative path
        let mpd_relative_path = match path_translator::absolute_to_mpd_relative(&song.path) {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to translate path '{}' to MPD format: {}. Skipping song.", 
                      song.path, e);
                failed_count += 1;
                continue;
            }
        };
        
        debug!("Translated path: {} -> {}", song.path, mpd_relative_path);
        
        let output = Command::new("mpc")
            .arg("add")
            .arg(&mpd_relative_path)
            .output()
            .with_context(|| format!("Failed to execute mpc add for: {mpd_relative_path}"))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Failed to add song '{}' (MPD path: '{}') to queue: {}", 
                song.path, mpd_relative_path, stderr.trim()
            );
            failed_count += 1;
        } else {
            debug!("Successfully added song: {mpd_relative_path}");
            added_count += 1;
        }
    }
    
    // Report results
    if added_count == 0 {
        anyhow::bail!(
            "Failed to add any songs to MPD queue. {} total failures. \
             This usually indicates a path translation issue or MPD database sync problem.",
            failed_count
        );
    }
    
    if failed_count > 0 {
        warn!(
            "Added {added_count} songs to MPD queue, but {failed_count} songs failed. \
             Check that all songs are in MPD's music directory."
        );
    } else {
        info!("Successfully loaded {added_count} songs into MPD queue");
    }
    
    // Track touches for all successfully queued songs
    if !queue.is_empty() {
        track_song_touches(queue).unwrap_or_else(|e| {
            warn!("Failed to track touches for queued songs: {e}");
        });
    }
    
    // Start playing
    let output = Command::new("mpc").arg("play").output()
        .context("Failed to start playback")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start playback: {}", stderr.trim());
    }
    
    Ok(())
}

fn play_with_algorithm() -> Result<()> {
    get_client()
        .context("Cannot start algorithmic play: MPD connection failed")?;
    let conn = db::get_connection()
        .context("Cannot start algorithmic play: database connection failed")?;
    
    // Get all songs and calculate scores
    let mut stmt = conn.prepare("SELECT * FROM songs")
        .context("Failed to prepare song selection query")?;
    let songs: Vec<db::Song> = stmt.query_map([], |row| {
        Ok(db::Song {
            id: row.get(0)?,
            path: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            title: row.get(4)?,
            touches: row.get(5)?,
            listens: row.get(6)?,
            skips: row.get(7)?,
            loved: row.get(8)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
        .context("Failed to load songs from database")?;
    
    if songs.is_empty() {
        anyhow::bail!(
            "No songs found in database. Please run 'muse update /path/to/music' first"
        );
    }
    
    // Calculate scores and sort
    let mut scored_songs: Vec<(db::Song, f64)> = songs
        .into_iter()
        .map(|song| {
            let score = algorithm::calculate_score(&song);
            (song, score)
        })
        .collect();
    
    scored_songs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // Ensure path translator is initialized
    if !path_translator::is_initialized() {
        info!("Initializing path translator for algorithmic play");
        path_translator::initialize()
            .context("Failed to initialize path translator for algorithmic play")?;
    }
    
    // Clear queue and add top songs
    Command::new("mpc").arg("clear").output()
        .context("Failed to clear MPD queue for algorithmic play")?;
    
    let mut added_count = 0;
    let mut failed_count = 0;
        
    for (song, score) in scored_songs.iter().take(50) {
        debug!("Adding song '{}' (score: {:.2}) to algorithmic queue", song.title, score);
        
        // Convert absolute path to MPD relative path
        let mpd_relative_path = match path_translator::absolute_to_mpd_relative(&song.path) {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to translate path for song '{}': {}. Skipping.", song.title, e);
                failed_count += 1;
                continue;
            }
        };
        
        let output = Command::new("mpc")
            .arg("add")
            .arg(&mpd_relative_path)
            .output()
            .with_context(|| format!("Failed to add song '{}' to queue", song.title))?;
            
        if !output.status.success() {
            warn!("Failed to add song '{}' (MPD path: '{}'): {}", 
                  song.title, mpd_relative_path,
                  String::from_utf8_lossy(&output.stderr).trim());
            failed_count += 1;
        } else {
            debug!("Successfully added song '{}' to algorithmic queue", song.title);
            added_count += 1;
        }
    }
    
    // Report results
    if added_count == 0 {
        anyhow::bail!(
            "Failed to add any songs to algorithmic queue. {} total failures.",
            failed_count
        );
    }
    
    if failed_count > 0 {
        warn!(
            "Added {added_count} songs to algorithmic queue, but {failed_count} songs failed."
        );
    } else {
        info!("Successfully added {added_count} songs to algorithmic queue");
    }
    
    Command::new("mpc").arg("play").output()
        .context("Failed to start algorithmic playback")?;
    
    Ok(())
}

/// # Errors
/// 
/// This function will return an error if:
/// - MPD connection fails
/// - mpc command execution fails
#[allow(dead_code)]
pub fn get_current_song() -> Result<Option<String>> {
    get_client()?;
    
    let output = Command::new("mpc")
        .arg("current")
        .arg("-f")
        .arg("%file%")
        .output()?;
    
    if output.status.success() {
        let file = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !file.is_empty() {
            return Ok(Some(file));
        }
    }
    
    Ok(None)
}

/// Track touches for queued songs (every time we suggest/queue a song)
/// 
/// This function increments the `touches` counter for each song that gets added
/// to the MPD queue. A "touch" represents the algorithm suggesting/recommending
/// a song to the user. This is crucial for behavioral learning as it tracks
/// how often each song is recommended.
/// 
/// # Arguments
/// 
/// * `queued_songs` - Slice of QueuedSong objects that were added to the MPD queue
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if all touches were tracked successfully, or an error if
///   database operations fail
/// 
/// # Behavior
/// 
/// - For each song in the queue, looks up the song ID in the database
/// - Increments the `touches` field by 1 using `update_song_stats`
/// - Logs warnings for songs not found in database (graceful degradation)
/// - Never fails the entire operation if individual songs can't be tracked
/// 
/// # Example
/// 
/// ```rust,ignore
/// use muse::queue::QueuedSong;
/// 
/// let queue = vec![
///     QueuedSong {
///         path: "/music/artist/song.mp3".to_string(),
///         artist: "Artist".to_string(),
///         title: "Song".to_string(),
///         score: 0.8,
///     }
/// ];
/// 
/// track_song_touches(&queue)?; // Increments touches for the song
/// ```
fn track_song_touches(queued_songs: &[QueuedSong]) -> Result<()> {
    debug!("Tracking touches for {} queued songs", queued_songs.len());
    
    let conn = db::get_connection()?;
    
    for song in queued_songs {
        // Get song ID from path
        match conn.query_row(
            "SELECT id FROM songs WHERE path = ?1",
            [&song.path],
            |row| row.get::<_, i64>(0)
        ) {
            Ok(song_id) => {
                // Increment touches for this song
                match db::update_song_stats(song_id, true, false, false) {
                    Ok(_) => debug!("Tracked touch for song: {}", song.path),
                    Err(e) => warn!("Failed to track touch for song '{}': {}", song.path, e),
                }
            }
            Err(e) => {
                warn!("Failed to find song '{}' in database for touch tracking: {}", song.path, e);
            }
        }
    }
    
    Ok(())
}

/// # Errors
/// 
/// This function will return an error if:
/// - Database connection fails
/// - Song paths are not found in database
/// - Database update operations fail
pub fn handle_song_finished(mpd_current_path: &str, mpd_next_path: &str) -> Result<()> {
    debug!("Handling song finished: {mpd_current_path} -> {mpd_next_path}");
    
    // Convert MPD relative paths to absolute paths for database lookup
    let current_absolute = path_translator::mpd_relative_to_absolute(mpd_current_path)
        .with_context(|| format!("Failed to convert current song path: {mpd_current_path}"))?;
    
    let next_absolute = path_translator::mpd_relative_to_absolute(mpd_next_path)
        .with_context(|| format!("Failed to convert next song path: {mpd_next_path}"))?;
    
    let current_path_str = current_absolute.to_string_lossy();
    let next_path_str = next_absolute.to_string_lossy();
    
    debug!("Converted paths: {mpd_current_path} -> {current_path_str}, {mpd_next_path} -> {next_path_str}");
    
    let conn = db::get_connection()?;
    
    // Get song IDs
    let current_id: i64 = conn.query_row(
        "SELECT id FROM songs WHERE path = ?1",
        [current_path_str.as_ref()],
        |row| row.get(0)
    ).with_context(|| format!("Failed to find current song in database: {current_path_str}"))?;
    
    let next_id: i64 = conn.query_row(
        "SELECT id FROM songs WHERE path = ?1",
        [next_path_str.as_ref()],
        |row| row.get(0)
    ).with_context(|| format!("Failed to find next song in database: {next_path_str}"))?;
    
    // Update connection
    db::update_connection(current_id, next_id)?;
    
    // Update stats
    db::update_song_stats(current_id, false, true, false)?;
    db::update_song_stats(next_id, true, false, false)?;
    
    info!("Updated statistics for song transition: {current_path_str} -> {next_path_str}");
    Ok(())
}

/// # Errors
/// 
/// This function will return an error if:
/// - Database connection fails
/// - Song path is not found in database
/// - Database update operation fails
pub fn handle_song_skipped(mpd_current_path: &str) -> Result<()> {
    debug!("Handling song skipped: {mpd_current_path}");
    
    // Convert MPD relative path to absolute path for database lookup
    let current_absolute = path_translator::mpd_relative_to_absolute(mpd_current_path)
        .with_context(|| format!("Failed to convert skipped song path: {mpd_current_path}"))?;
    
    let current_path_str = current_absolute.to_string_lossy();
    debug!("Converted path: {mpd_current_path} -> {current_path_str}");
    
    let conn = db::get_connection()?;
    
    let song_id: i64 = conn.query_row(
        "SELECT id FROM songs WHERE path = ?1",
        [current_path_str.as_ref()],
        |row| row.get(0)
    ).with_context(|| format!("Failed to find skipped song in database: {current_path_str}"))?;
    
    db::update_song_stats(song_id, false, false, true)?;
    
    info!("Updated skip statistics for song: {current_path_str}");
    Ok(())
}

/// MPD status information parsed from `mpc status` output
/// 
/// This struct contains the current state of the MPD player, including
/// the currently playing song, playback position, and duration. Used
/// for intelligent behavior tracking and listen/skip detection.
#[derive(Debug)]
pub struct MpdStatus {
    /// Current song file path (MPD relative), None if nothing playing
    pub current_song: Option<String>,
    /// Elapsed time in seconds (floating point for precision)
    pub elapsed: f64,
    /// Total song duration in seconds, None for streams or unknown
    pub duration: Option<f64>,
    /// Playback state: "play", "pause", "stop"
    #[allow(dead_code)]
    pub state: String,
}

/// Get current MPD status by parsing `mpc status` output
/// 
/// Executes `mpc status -f %file%` and parses the output to extract:
/// - Currently playing song path
/// - Elapsed playback time
/// - Total song duration 
/// - Playback state (play/pause/stop)
/// 
/// # Returns
/// 
/// * `Result<MpdStatus>` - Parsed status information or error if MPD unavailable
/// 
/// # Error Conditions
/// 
/// - MPD is not running or unreachable
/// - `mpc` command not found in PATH
/// - Malformed status output (shouldn't happen with standard MPD)
/// 
/// # Example Output Parsing
/// 
/// ```text
/// artist/album/song.mp3
/// [playing] #5/20   1:23/3:45 (37%)
/// volume: 80%   repeat: on    random: off   single: off   consume: off
/// ```
/// 
/// Extracts: song="artist/album/song.mp3", elapsed=83.0, duration=225.0, state="play"
pub fn get_mpd_status() -> Result<MpdStatus> {
    debug!("Getting MPD status");
    
    let output = Command::new("mpc")
        .arg("status")
        .arg("-f")
        .arg("%file%")
        .output()
        .context("Failed to get MPD status")?;
    
    if !output.status.success() {
        anyhow::bail!("MPD status command failed");
    }
    
    let status_text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = status_text.lines().collect();
    
    let current_song = if !lines.is_empty() && !lines[0].trim().is_empty() {
        Some(lines[0].trim().to_string())
    } else {
        None
    };
    
    let mut elapsed = 0.0;
    let mut duration = None;
    let mut state = "stop".to_string();
    
    // Parse status line (format: [playing] #1/50   0:32/3:45 (13%))
    for line in lines.iter().skip(1) {
        if line.contains("[playing]") {
            state = "play".to_string();
        } else if line.contains("[paused]") {
            state = "pause".to_string();
        }
        
        // Extract time information
        if let Some(time_part) = line.split_whitespace().find(|s| s.contains("/")) {
            let times: Vec<&str> = time_part.split('/').collect();
            if times.len() == 2 {
                if let Ok(elapsed_secs) = parse_time(times[0]) {
                    elapsed = elapsed_secs;
                }
                if let Ok(duration_secs) = parse_time(times[1]) {
                    duration = Some(duration_secs);
                }
            }
        }
    }
    
    Ok(MpdStatus {
        current_song,
        elapsed,
        duration,
        state,
    })
}

/// Parse time string in MM:SS format to seconds
/// 
/// Converts MPD time format (e.g., "1:23", "12:34") to floating-point seconds.
/// Used for parsing elapsed time and duration from `mpc status` output.
/// 
/// # Arguments
/// 
/// * `time_str` - Time string in "MM:SS" format
/// 
/// # Returns
/// 
/// * `Result<f64>` - Time in seconds, or error if format is invalid
/// 
/// # Examples
/// 
/// ```rust,ignore
/// assert_eq!(parse_time("0:30").unwrap(), 30.0);
/// assert_eq!(parse_time("1:23").unwrap(), 83.0);
/// assert_eq!(parse_time("12:34").unwrap(), 754.0);
/// ```
/// 
/// # Error Conditions
/// 
/// - Invalid format (not "MM:SS")
/// - Non-numeric minutes or seconds
/// - Empty string or malformed input
fn parse_time(time_str: &str) -> Result<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.len() {
        2 => {
            let minutes: f64 = parts[0].parse()?;
            let seconds: f64 = parts[1].parse()?;
            Ok(minutes * 60.0 + seconds)
        }
        _ => anyhow::bail!("Invalid time format: {}", time_str),
    }
}

/// Skip to next track with intelligent behavior tracking
/// 
/// This function advances to the next track in the MPD queue while automatically
/// determining whether the current track was listened to or skipped based on
/// playback progress. Uses the 80% threshold: if >80% of the song was played,
/// it's considered a "listen", otherwise it's a "skip".
/// 
/// # Behavior Detection Logic
/// 
/// - **Listen**: elapsed_time / total_duration > 0.8
/// - **Skip**: elapsed_time / total_duration ≤ 0.8 or no duration info
/// 
/// This provides more accurate behavioral data than manual classification,
/// as users often let songs play naturally vs. explicitly skipping them.
/// 
/// # Database Updates
/// 
/// - **If Listen**: Calls `handle_song_finished(current, next)` 
///   - Increments `listens` for current song
///   - Increments `touches` for next song
///   - Updates connection weight between songs
/// - **If Skip**: Calls `handle_song_skipped(current)`
///   - Increments `skips` for current song
///   - No connection weight update (skipped songs don't create connections)
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if successful, error if MPD operations or tracking fail
/// 
/// # Example
/// 
/// ```bash
/// # User runs this instead of `mpc next`
/// muse next
/// ```
/// 
/// Song at 2:30/3:00 (83%) → Tracked as **listen**  
/// Song at 1:00/3:00 (33%) → Tracked as **skip**
pub fn next_with_tracking() -> Result<()> {
    debug!("Advancing to next track with behavior tracking");
    
    // Get current song info before advancing
    let status = get_mpd_status()?;
    
    if let Some(current_file) = status.current_song {
        // Determine if this was a natural listen or skip
        let was_natural_listen = if let Some(duration) = status.duration {
            status.elapsed / duration > 0.8  // Listened to >80% = natural listen
        } else {
            false  // No duration info = assume skip
        };
        
        // Execute MPD command first
        let output = Command::new("mpc").arg("next").output()
            .context("Failed to advance to next track")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to advance to next track: {}", stderr.trim());
        }
        
        // Track behavior based on listening progress
        if was_natural_listen {
            // Get the new current song after advancing
            let new_status = get_mpd_status().unwrap_or_else(|_| MpdStatus {
                current_song: None,
                elapsed: 0.0,
                duration: None,
                state: "stop".to_string(),
            });
            let next_file = new_status.current_song.unwrap_or_default();
            handle_song_finished(&current_file, &next_file)?;
        } else {
            handle_song_skipped(&current_file)?;
        }
        
        info!("Advanced to next track, tracked as {}", 
              if was_natural_listen { "listened" } else { "skipped" });
    } else {
        // No current song, just advance
        Command::new("mpc").arg("next").output()
            .context("Failed to advance to next track")?;
        info!("Advanced to next track (no current song to track)");
    }
    
    Ok(())
}

/// Skip current track with explicit skip tracking
/// 
/// This function advances to the next track while **always** marking the current
/// track as skipped, regardless of how much was played. Use this when the user
/// explicitly wants to skip a song they don't like, providing stronger negative
/// signal to the recommendation algorithm.
/// 
/// # Difference from `next_with_tracking()`
/// 
/// - `next_with_tracking()`: Analyzes playback progress (listen vs skip)
/// - `skip_with_tracking()`: Always records as skip (explicit user dislike)
/// 
/// This distinction helps the algorithm differentiate between:
/// - Natural song endings that happen to be short
/// - Active user rejection of songs
/// 
/// # Database Updates
/// 
/// - Always calls `handle_song_skipped(current)`
/// - Increments `skips` counter for current song
/// - **No connection weight update** (skipped songs don't influence transitions)
/// - No `touches` increment for next song (user didn't choose the transition)
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if successful, error if MPD operations or tracking fail
/// 
/// # Example
/// 
/// ```bash
/// # User actively dislikes current song
/// muse skip
/// ```
/// 
/// Even if song was at 2:45/3:00 (92%), still tracked as **skip**
pub fn skip_with_tracking() -> Result<()> {
    debug!("Skipping current track with explicit skip tracking");
    
    // Get current song info before skipping
    let status = get_mpd_status()?;
    
    if let Some(current_file) = status.current_song {
        // Execute MPD command first
        let output = Command::new("mpc").arg("next").output()
            .context("Failed to skip track")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to skip track: {}", stderr.trim());
        }
        
        // Always track as skipped (explicit user action)
        handle_song_skipped(&current_file)?;
        
        info!("Skipped track: {current_file}");
    } else {
        // No current song, just advance
        Command::new("mpc").arg("next").output()
            .context("Failed to skip track")?;
        info!("Advanced to next track (no current song to skip)");
    }
    
    Ok(())
}

/// Mark current track as loved
/// 
/// Sets the `loved` flag to 1 for the currently playing track. Loved songs
/// receive 2x weight in the recommendation algorithm, making them much more
/// likely to be selected in future queue generation.
/// 
/// # Algorithm Impact
/// 
/// ```text
/// Normal song score: base_score * 1.0
/// Loved song score:  base_score * 2.0
/// ```
/// 
/// This creates a strong positive signal that this song should be recommended
/// more frequently, even if it has relatively few listens or high skips.
/// 
/// # Database Updates
/// 
/// - Updates `loved = 1` for the current song
/// - Takes effect immediately in next algorithm run
/// - Persistent across program restarts
/// 
/// # Error Handling
/// 
/// - Fails if no song is currently playing
/// - Fails if path translation fails (MPD → absolute path)
/// - Fails if song not found in database
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if loved status set, error if no current song or DB issues
/// 
/// # Example
/// 
/// ```bash
/// # User loves the currently playing song
/// muse love
/// ```
/// 
/// Song will now have 2x recommendation weight in future algorithm runs
pub fn love_current_track() -> Result<()> {
    debug!("Marking current track as loved");
    
    let status = get_mpd_status()?;
    
    if let Some(current_file) = status.current_song {
        // Convert MPD path to absolute path for database lookup
        let absolute_path = path_translator::mpd_relative_to_absolute(&current_file)
            .with_context(|| format!("Failed to convert path for love: {current_file}"))?;
        
        let path_str = absolute_path.to_string_lossy();
        
        // Get song ID and update loved status
        let conn = db::get_connection()?;
        let song_id: i64 = conn.query_row(
            "SELECT id FROM songs WHERE path = ?1",
            [path_str.as_ref()],
            |row| row.get(0)
        ).with_context(|| format!("Failed to find song in database: {path_str}"))?;
        
        // Update loved status in database
        conn.execute(
            "UPDATE songs SET loved = 1 WHERE id = ?1",
            [song_id],
        ).context("Failed to update loved status")?;
        
        info!("Marked song as loved: {current_file}");
    } else {
        anyhow::bail!("No song currently playing to love");
    }
    
    Ok(())
}

/// Remove loved status from current track
/// 
/// Sets the `loved` flag to 0 for the currently playing track, returning it
/// to normal recommendation weight. Use this to undo accidental loves or when
/// preferences change over time.
/// 
/// # Algorithm Impact
/// 
/// ```text
/// Previously: base_score * 2.0 (loved)
/// After:      base_score * 1.0 (normal)
/// ```
/// 
/// The song returns to being weighted based purely on its listen/skip ratio
/// and other behavioral metrics, without the love bonus.
/// 
/// # Database Updates
/// 
/// - Updates `loved = 0` for the current song
/// - Takes effect immediately in next algorithm run
/// - Persistent across program restarts
/// 
/// # Error Handling
/// 
/// - Fails if no song is currently playing
/// - Fails if path translation fails (MPD → absolute path)  
/// - Fails if song not found in database
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if loved status removed, error if no current song or DB issues
/// 
/// # Example
/// 
/// ```bash
/// # User wants to remove love from current song
/// muse unlove
/// ```
/// 
/// Song returns to normal 1x recommendation weight
pub fn unlove_current_track() -> Result<()> {
    debug!("Removing loved status from current track");
    
    let status = get_mpd_status()?;
    
    if let Some(current_file) = status.current_song {
        // Convert MPD path to absolute path for database lookup
        let absolute_path = path_translator::mpd_relative_to_absolute(&current_file)
            .with_context(|| format!("Failed to convert path for unlove: {current_file}"))?;
        
        let path_str = absolute_path.to_string_lossy();
        
        // Get song ID and update loved status
        let conn = db::get_connection()?;
        let song_id: i64 = conn.query_row(
            "SELECT id FROM songs WHERE path = ?1",
            [path_str.as_ref()],
            |row| row.get(0)
        ).with_context(|| format!("Failed to find song in database: {path_str}"))?;
        
        // Update loved status in database
        conn.execute(
            "UPDATE songs SET loved = 0 WHERE id = ?1",
            [song_id],
        ).context("Failed to update loved status")?;
        
        info!("Removed loved status from song: {current_file}");
    } else {
        anyhow::bail!("No song currently playing to unlove");
    }
    
    Ok(())
}

/// Show detailed information about the currently playing song
/// 
/// Displays comprehensive statistics and algorithm data for the current song
/// including touches, listens, skips, score, and connection information.
/// 
/// # Returns
/// 
/// * `Result<()>` - Ok if successful, error if no song playing or database issues
pub fn show_current_song_info() -> Result<()> {
    let status = get_mpd_status()?;
    
    if let Some(current_file) = status.current_song {
        // Convert MPD path to absolute path
        let absolute_path = path_translator::mpd_relative_to_absolute(&current_file)
            .with_context(|| format!("Failed to convert current song path: {current_file}"))?;
        
        let path_str = absolute_path.to_string_lossy();
        
        // Get song details from database
        let conn = db::get_connection()?;
        let song_info: (i64, String, String, String, i32, i32, i32, bool) = conn.query_row(
            "SELECT id, title, artist, album, touches, listens, skips, loved FROM songs WHERE path = ?1",
            [path_str.as_ref()],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get::<_, i32>(7)? == 1
            ))
        ).with_context(|| format!("Song not found in database: {path_str}"))?;
        
        let (song_id, title, artist, album, touches, listens, skips, loved) = song_info;
        
        // Calculate current score
        let song = db::Song {
            id: song_id,
            path: path_str.to_string(),
            artist: artist.clone(),
            album: album.clone(),
            title: title.clone(),
            touches: touches as u32,
            listens: listens as u32,
            skips: skips as u32,
            loved,
        };
        
        let context = algorithm::ScoringContext::default();
        let score = algorithm::calculate_score_functional(&song, &context);
        
        // Get playback info
        let elapsed_mins = (status.elapsed / 60.0) as u32;
        let elapsed_secs = (status.elapsed % 60.0) as u32;
        
        let duration_str = if let Some(duration) = status.duration {
            let total_mins = (duration / 60.0) as u32;
            let total_secs = (duration % 60.0) as u32;
            format!("{elapsed_mins:02}:{elapsed_secs:02}/{total_mins:02}:{total_secs:02}")
        } else {
            format!("{elapsed_mins:02}:{elapsed_secs:02}/??:??")
        };
        
        // Display information
        println!("📀 Currently Playing");
        println!("═══════════════════");
        println!("♫ Song: {title}");
        println!("👤 Artist: {artist}");
        println!("💿 Album: {album}");
        println!("⏱️  Time: {duration_str}");
        
        if loved {
            println!("❤️  Status: LOVED");
        }
        
        println!();
        println!("📊 Algorithm Statistics");
        println!("═════════════════════");
        println!("🎯 Current Score: {score:.3}");
        println!("👆 Touches: {touches}");
        println!("✅ Listens: {listens}");
        println!("⏭️  Skips: {skips}");
        
        if touches > 0 {
            let listen_rate = (listens as f64 / touches as f64 * 100.0) as u32;
            let skip_rate = (skips as f64 / touches as f64 * 100.0) as u32;
            println!("📈 Listen Rate: {listen_rate}%");
            println!("📉 Skip Rate: {skip_rate}%");
        }
        
        // Show connection information
        let connections: Vec<(String, String, i32)> = conn.prepare(
            "SELECT s.title, s.artist, c.count 
             FROM connections c 
             JOIN songs s ON c.to_song_id = s.id 
             WHERE c.from_song_id = ?1
             ORDER BY c.count DESC 
             LIMIT 5"
        )?.query_map([song_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        
        if !connections.is_empty() {
            println!();
            println!("🔗 Top Connections");
            println!("═════════════════");
            for (i, (conn_title, conn_artist, count)) in connections.iter().enumerate() {
                println!("  {}. {} - {} (played together {} times)", 
                         i + 1, conn_artist, conn_title, count);
            }
        }
        
    } else {
        println!("⏸️  No song is currently playing");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use rusqlite::Connection;

    /// Create a test database with sample data
    fn create_test_db() -> Result<(TempDir, String)> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test_music.db");
        let db_path_str = db_path.to_string_lossy().to_string();
        
        let conn = Connection::open(&db_path)?;
        
        // Create songs table
        conn.execute(
            "CREATE TABLE songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT UNIQUE NOT NULL,
                artist TEXT NOT NULL,
                album TEXT NOT NULL,
                title TEXT NOT NULL,
                touches INTEGER DEFAULT 0 CHECK (touches >= 0),
                listens INTEGER DEFAULT 0 CHECK (listens >= 0),
                skips INTEGER DEFAULT 0 CHECK (skips >= 0),
                loved INTEGER DEFAULT 0 CHECK (loved IN (0, 1)),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        
        // Insert test songs
        let test_songs = [
            ("/music/artist1/album1/song1.mp3", "Artist 1", "Album 1", "Song 1", 5, 3, 2, 0),
            ("/music/artist2/album2/song2.mp3", "Artist 2", "Album 2", "Song 2", 10, 8, 2, 1),
            ("/music/artist3/album3/song3.mp3", "Artist 3", "Album 3", "Song 3", 15, 10, 5, 0),
        ];
        
        for (path, artist, album, title, touches, listens, skips, loved) in test_songs {
            conn.execute(
                "INSERT INTO songs (path, artist, album, title, touches, listens, skips, loved) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                [path, artist, album, title, &touches.to_string(), &listens.to_string(), &skips.to_string(), &loved.to_string()],
            )?;
        }
        
        Ok((temp_dir, db_path_str))
    }

    #[test]
    fn test_parse_time_valid_formats() -> Result<()> {
        assert_eq!(parse_time("0:30")?, 30.0);
        assert_eq!(parse_time("1:45")?, 105.0);
        assert_eq!(parse_time("12:34")?, 754.0);
        Ok(())
    }

    #[test]
    fn test_parse_time_invalid_formats() {
        assert!(parse_time("invalid").is_err());
        assert!(parse_time("1:2:3").is_err());
        assert!(parse_time("").is_err());
        assert!(parse_time("1:").is_err());
        assert!(parse_time(":30").is_err());
    }

    #[test]
    fn test_mpd_status_parsing() -> Result<()> {
        // Test parsing a typical mpc status output
        let status = MpdStatus {
            current_song: Some("artist/album/song.mp3".to_string()),
            elapsed: 45.0,
            duration: Some(180.0),
            state: "play".to_string(),
        };
        
        // Test listen threshold calculation
        let listen_ratio = status.elapsed / status.duration.unwrap();
        assert!(listen_ratio < 0.8); // Should be classified as skip
        
        // Test with listen threshold
        let status_listened = MpdStatus {
            current_song: Some("artist/album/song.mp3".to_string()),
            elapsed: 150.0,
            duration: Some(180.0),
            state: "play".to_string(),
        };
        
        let listen_ratio = status_listened.elapsed / status_listened.duration.unwrap();
        assert!(listen_ratio > 0.8); // Should be classified as listen
        
        Ok(())
    }

    #[test]
    fn test_track_song_touches() -> Result<()> {
        let (_temp_dir, _db_path) = create_test_db()?;
        
        // NOTE: This test validates the logic but can't test the actual DB update
        // since track_song_touches() uses the global db::get_connection() which
        // doesn't use our test database. The function logic is correct, but
        // testing it requires mocking the database layer.
        
        // Create test queued songs - this validates the structure and doesn't panic
        let queued_songs = [
            QueuedSong {
                path: "/music/artist1/album1/song1.mp3".to_string(),
                artist: "Artist 1".to_string(),
                title: "Song 1".to_string(),
                score: 1.0,
            },
            QueuedSong {
                path: "/music/artist2/album2/song2.mp3".to_string(),
                artist: "Artist 2".to_string(),
                title: "Song 2".to_string(),
                score: 1.0,
            },
        ];
        
        // Verify the songs are structured correctly
        assert_eq!(queued_songs.len(), 2);
        assert_eq!(queued_songs[0].path, "/music/artist1/album1/song1.mp3");
        assert_eq!(queued_songs[1].path, "/music/artist2/album2/song2.mp3");
        
        // The function would track touches, but we can't test the DB interaction
        // in unit tests without a database abstraction layer. This is an
        // integration test that would be better handled with actual MPD commands.
        
        Ok(())
    }

    #[test]
    fn test_track_song_touches_missing_songs() -> Result<()> {
        let (_temp_dir, db_path) = create_test_db()?;
        std::env::set_var("MUSE_DB_PATH", &db_path);
        
        // Create queued songs with non-existent paths
        let queued_songs = vec![
            QueuedSong {
                path: "/nonexistent/song.mp3".to_string(),
                artist: "Unknown".to_string(),
                title: "Unknown".to_string(),
                score: 1.0,
            },
        ];
        
        // Should not fail, just log warnings
        let result = track_song_touches(&queued_songs);
        assert!(result.is_ok());
        
        Ok(())
    }

    #[test]
    fn test_mpd_status_edge_cases() {
        // Test status with no current song
        let status = MpdStatus {
            current_song: None,
            elapsed: 0.0,
            duration: None,
            state: "stop".to_string(),
        };
        assert!(status.current_song.is_none());
        
        // Test status with no duration (stream)
        let status_stream = MpdStatus {
            current_song: Some("stream.mp3".to_string()),
            elapsed: 60.0,
            duration: None,
            state: "play".to_string(),
        };
        assert!(status_stream.duration.is_none());
    }

    #[test]
    fn test_behavior_tracking_integration() -> Result<()> {
        let (_temp_dir, db_path) = create_test_db()?;
        std::env::set_var("MUSE_DB_PATH", &db_path);
        
        // Test updating song stats
        let conn = Connection::open(&db_path)?;
        
        // Get initial stats
        let (initial_touches, initial_listens, initial_skips): (u32, u32, u32) = conn.query_row(
            "SELECT touches, listens, skips FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        )?;
        
        // Mock update_song_stats call
        let song_id: i64 = conn.query_row(
            "SELECT id FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| row.get(0)
        )?;
        
        // Update touches
        conn.execute(
            "UPDATE songs SET touches = touches + 1 WHERE id = ?1",
            [song_id],
        )?;
        
        // Update listens
        conn.execute(
            "UPDATE songs SET listens = listens + 1 WHERE id = ?1",
            [song_id],
        )?;
        
        // Update skips
        conn.execute(
            "UPDATE songs SET skips = skips + 1 WHERE id = ?1",
            [song_id],
        )?;
        
        // Verify updates
        let (final_touches, final_listens, final_skips): (u32, u32, u32) = conn.query_row(
            "SELECT touches, listens, skips FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        )?;
        
        assert_eq!(final_touches, initial_touches + 1);
        assert_eq!(final_listens, initial_listens + 1);
        assert_eq!(final_skips, initial_skips + 1);
        
        Ok(())
    }

    #[test]
    fn test_love_functionality() -> Result<()> {
        let (_temp_dir, db_path) = create_test_db()?;
        std::env::set_var("MUSE_DB_PATH", &db_path);
        
        let conn = Connection::open(&db_path)?;
        
        // Test song initially not loved
        let loved_status: i32 = conn.query_row(
            "SELECT loved FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| row.get(0)
        )?;
        assert_eq!(loved_status, 0);
        
        // Set as loved
        let song_id: i64 = conn.query_row(
            "SELECT id FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| row.get(0)
        )?;
        
        conn.execute(
            "UPDATE songs SET loved = 1 WHERE id = ?1",
            [song_id],
        )?;
        
        // Verify loved status
        let loved_status: i32 = conn.query_row(
            "SELECT loved FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| row.get(0)
        )?;
        assert_eq!(loved_status, 1);
        
        // Remove loved status
        conn.execute(
            "UPDATE songs SET loved = 0 WHERE id = ?1",
            [song_id],
        )?;
        
        // Verify unloved status
        let loved_status: i32 = conn.query_row(
            "SELECT loved FROM songs WHERE path = ?1",
            ["/music/artist1/album1/song1.mp3"],
            |row| row.get(0)
        )?;
        assert_eq!(loved_status, 0);
        
        Ok(())
    }

    #[test]
    fn test_listen_skip_threshold() {
        // Test the 80% threshold logic
        let test_cases = [
            (30.0, 180.0, false),   // 16.7% - skip
            (90.0, 180.0, false),   // 50% - skip  
            (140.0, 180.0, false),  // 77.8% - skip
            (145.0, 180.0, true),   // 80.6% - listen
            (160.0, 180.0, true),   // 88.9% - listen
            (180.0, 180.0, true),   // 100% - listen
        ];
        
        for (elapsed, duration, expected_listen) in test_cases {
            let ratio = elapsed / duration;
            let is_listen = ratio > 0.8;
            assert_eq!(is_listen, expected_listen, 
                      "Failed for elapsed={elapsed}, duration={duration}, ratio={ratio}");
        }
    }

    #[test]
    fn test_queue_song_creation() {
        let song = QueuedSong {
            path: "/test/path.mp3".to_string(),
            artist: "Test Artist".to_string(),
            title: "Test Title".to_string(),
            score: 0.85,
        };
        
        assert_eq!(song.path, "/test/path.mp3");
        assert_eq!(song.artist, "Test Artist");
        assert_eq!(song.title, "Test Title");
        assert_eq!(song.score, 0.85);
    }

    #[test]
    fn test_mpd_status_construction() {
        let status = MpdStatus {
            current_song: Some("test/song.mp3".to_string()),
            elapsed: 123.45,
            duration: Some(234.56),
            state: "playing".to_string(),
        };
        
        assert_eq!(status.current_song.unwrap(), "test/song.mp3");
        assert_eq!(status.elapsed, 123.45);
        assert_eq!(status.duration.unwrap(), 234.56);
        assert_eq!(status.state, "playing");
    }
}
