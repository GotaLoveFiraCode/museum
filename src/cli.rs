//! # Command-Line Interface Module
//!
//! This module defines the command-line interface for Muse using Clap derive macros.
//! It provides a type-safe way to parse command-line arguments and route them to
//! appropriate functionality.
//!
//! ## Commands
//!
//! - `update`: Scan filesystem and add new songs to database
//! - `list`: Display all catalogued songs with statistics
//! - `play`: Start playback in shuffle or algorithm mode
//! - `current`: Generate dual-path queue based on starting song
//! - `thread`: Generate single-path queue based on starting song  
//! - `stream`: Generate training queue with randomness
//!
//! ## Examples
//!
//! ```bash
//! muse update /home/user/Music
//! muse play algorithm
//! muse current "Miles Davis"
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Shell types supported for completion generation
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    PowerShell,
    /// Elvish shell
    Elvish,
}

/// Main application arguments structure.
/// 
/// Uses Clap derive macros to automatically generate argument parsing,
/// help text, and validation. The main structure contains only a subcommand
/// since all functionality is accessed through specific commands.
#[derive(Parser)]
#[command(name = "muse")]
#[command(about = "Muse: Unleashing Music - Offline music suggestions & playlists")]
#[command(version)]
pub struct Args {
    
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Command,
}

/// Enumeration of all available subcommands.
/// 
/// Each variant corresponds to a major piece of functionality in Muse.
/// Command arguments are embedded directly in the enum variants for
/// type safety and automatic validation.
#[derive(Subcommand)]
pub enum Command {
    /// Initialize database from music directory (full scan)
    /// 
    /// Performs a complete scan of the music directory and creates/overwrites
    /// the database with all found music files. This mirrors the MPD database
    /// content and extracts metadata (artist, album, title) from audio files.
    /// 
    /// Supported formats: FLAC, MP3, OGG, M4A, WAV
    InitDb {
        /// Path to the music directory to scan
        /// 
        /// Should be the root of your music collection. The scanner will
        /// recursively find all supported audio files in subdirectories.
        path: PathBuf,
        
        /// Force overwrite existing database
        /// 
        /// If specified, will delete and recreate the database even if it
        /// already exists. Without this flag, init-db will fail if database exists.
        #[arg(long)]
        force: bool,
        
        /// Skip metadata extraction (paths only)
        /// 
        /// If specified, only file paths will be stored without extracting
        /// ID3 tags. This is faster but provides less search functionality.
        #[arg(long)]
        no_metadata: bool,
    },
    
    /// Update database with new files (incremental)
    /// 
    /// Scans the music directory and adds only new files not already in
    /// the database. This is much faster than init-db for adding new music
    /// to an existing collection.
    /// 
    /// Supported formats: FLAC, MP3, OGG, M4A, WAV
    Update {
        /// Path to the music directory to scan
        /// 
        /// Should be the root of your music collection. The scanner will
        /// recursively find all supported audio files in subdirectories.
        path: PathBuf,
        
        /// Maximum scan depth
        /// 
        /// Limits how deep the scanner will recurse into subdirectories.
        /// Lower values improve performance for large collections.
        #[arg(long, default_value = "10")]
        scan_depth: u32,
        
        /// Remove entries for missing files
        /// 
        /// If specified, will remove database entries for files that no
        /// longer exist in the filesystem. Useful after reorganizing music.
        #[arg(long)]
        remove_missing: bool,
    },
    
    /// List all songs in the database
    /// 
    /// Displays all catalogued songs with their statistics including
    /// touches (times suggested), listens (complete plays), and skips.
    /// Output is sorted alphabetically by artist, then album, then title.
    List,
    
    /// Play music using MPD
    /// 
    /// Starts playback through MPD in the specified mode. Algorithm mode
    /// uses Muse's intelligent scoring system, while shuffle mode uses
    /// random playback.
    Play {
        /// Play mode: "shuffle" or "algorithm"
        /// 
        /// - "algorithm": Uses Muse's scoring system to select top songs
        /// - "shuffle": Random playback with MPD's built-in shuffle
        #[arg(default_value = "algorithm")]
        mode: String,
    },
    
    /// Generate and play a "current" queue based on a song
    /// 
    /// Creates a dual-path queue that follows the two strongest connections
    /// from the starting song, then interleaves them for variety.
    /// Queue length: 9-27 songs depending on available connections.
    /// 
    /// Best for: Daily background listening with variety
    Current {
        /// Song name to base the current queue on
        /// 
        /// Can be partial song title, artist name, or album name.
        /// Uses fuzzy matching to find the best match in the database.
        #[arg(value_hint = clap::ValueHint::Other)]
        song: String,
        
        /// Enable verbose output showing algorithm decisions
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Generate and play a "thread" queue based on a song
    /// 
    /// Creates a single-path queue that follows the strongest connection
    /// chain from the starting song for a coherent musical journey.
    /// Queue length: 9-27 songs depending on available connections.
    /// 
    /// Best for: Focused listening with consistent mood/genre
    Thread {
        /// Song name to base the thread queue on
        /// 
        /// Can be partial song title, artist name, or album name.
        /// Uses fuzzy matching to find the best match in the database.
        #[arg(value_hint = clap::ValueHint::Other)]
        song: String,
        
        /// Enable verbose output showing algorithm decisions
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Generate and play a "stream" queue based on a song
    /// 
    /// Creates a 30-song queue with controlled randomness for algorithm
    /// training. Includes exploration of less-connected songs to help
    /// build the connection graph.
    /// 
    /// Best for: Training the algorithm and discovering forgotten music
    Stream {
        /// Song name to base the stream queue on
        /// 
        /// Can be partial song title, artist name, or album name.
        /// Uses fuzzy matching to find the best match in the database.
        #[arg(value_hint = clap::ValueHint::Other)]
        song: String,
        
        /// Enable verbose output showing algorithm decisions
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Generate shell completions
    /// 
    /// Generates completion scripts for various shells to enable tab completion
    /// of commands, subcommands, and song names from the database.
    /// 
    /// Usage: muse completion bash > ~/.local/share/bash-completion/completions/muse
    Completion {
        /// Shell to generate completions for
        shell: Shell,
    },
    
    /// Generate enhanced completion with song name completion
    /// 
    /// Generates an enhanced completion script that includes dynamic song name
    /// completion for current, thread, and stream commands.
    /// 
    /// Usage: muse completion-enhanced bash > ~/.local/share/bash-completion/completions/muse
    /// Usage: muse completion-enhanced fish > ~/.config/fish/completions/muse.fish
    CompletionEnhanced {
        /// Shell to generate enhanced completions for (currently bash and fish supported)
        shell: Shell,
    },
    
    /// List available songs for completion (hidden command)
    #[command(hide = true)]
    CompleteSongs,
    
    /// List available songs for fish shell completion (hidden command)
    #[command(hide = true)]
    CompleteSongsFish,
    
    /// Skip to next track with behavior tracking
    /// 
    /// Advances to the next track in the MPD queue and records the current
    /// track as skipped for algorithmic learning. More accurate than mpc next
    /// as it tracks user behavior for recommendations.
    Next,
    
    /// Skip current track with explicit skip tracking
    /// 
    /// Same as 'next' but explicitly marks the current track as skipped
    /// rather than naturally finished. Useful for training the algorithm
    /// about songs you actively dislike.
    Skip,
    
    /// Mark current track as loved
    /// 
    /// Sets the 'loved' flag on the currently playing track, which increases
    /// its weight in algorithmic recommendations. Loved songs are 2x more
    /// likely to be selected.
    Love,
    
    /// Remove loved status from current track  
    /// 
    /// Removes the 'loved' flag from the currently playing track, returning
    /// it to normal recommendation weight.
    Unlove,
    
    /// Show detailed information about the current song
    /// 
    /// Displays the currently playing song's statistics including:
    /// - Touches, listens, skips counts
    /// - Current algorithm score
    /// - Connection strength to next song
    /// - Loved status
    Info,
    
    /// Manage the behavior tracking daemon
    /// 
    /// The daemon monitors MPD events in real-time to track user behavior
    /// (listens, skips, touches) automatically for all playback, not just
    /// when using muse commands.
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    
}

/// Daemon management actions
#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    /// Start the behavior tracking daemon
    /// 
    /// Starts a background process that monitors MPD events and updates
    /// the database with listen/skip statistics automatically.
    Start,
    
    /// Stop the running daemon
    /// 
    /// Stops the background behavior tracking daemon if it's running.
    Stop,
    
    /// Check daemon status
    /// 
    /// Reports whether the behavior tracking daemon is currently running.
    Status,
}