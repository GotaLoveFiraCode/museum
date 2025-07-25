//! # Muse - Intelligent Music Player
//! 
//! Muse is an offline music player that learns from your listening habits to provide
//! personalized music suggestions. This version uses MPD for playbook and focuses on
//! CLI-only operation for maximum efficiency.
//! 
//! ## Architecture
//! 
//! - `cli`: Command-line interface definitions
//! - `db`: SQLite database operations and schema
//! - `algorithm`: Scoring algorithms (functional implementation)
//! - `mpd_client`: MPD integration via mpc command-line tool
//! - `queue`: Queue generation logic (Current, Thread, Stream)
//! - `config`: Configuration and data directory management
//! 
//! ## Usage
//! 
//! ```bash
//! # Update music database
//! muse update /path/to/music
//! 
//! # List songs
//! muse list
//! 
//! # Play with algorithm
//! muse play algorithm
//! 
//! # Generate specific queue types
//! muse current "Song Name"
//! muse thread "Artist Name"  
//! muse stream "Album Name"
//! ```

mod cli;
mod completion;
mod db;
mod algorithm;
mod mpd_client;
mod mpd_config;
mod path_translator;
mod queue;
mod config;

use anyhow::Result;
use clap::{Parser, CommandFactory};
use log::{info, debug};

/// Initialize path translator for commands that interact with MPD
/// This provides better error messages by doing initialization early
fn ensure_path_translator_ready() -> Result<()> {
    if !path_translator::is_initialized() {
        debug!("Initializing path translator for MPD command");
        path_translator::initialize()
            .map_err(|e| {
                eprintln!("Failed to initialize MPD path translation:");
                eprintln!("  {e}");
                eprintln!();
                eprintln!("This error typically means:");
                eprintln!("  1. MPD is not running or not accessible");
                eprintln!("  2. MPD configuration file is missing or invalid");
                eprintln!("  3. Music directory is not properly configured");
                eprintln!("  4. Database is empty or out of sync with MPD");
                eprintln!();
                eprintln!("To fix this:");
                eprintln!("  1. Start MPD: systemctl start mpd");
                eprintln!("  2. Check MPD config: ~/.config/mpd/mpd.conf");
                eprintln!("  3. Update music database: muse update /path/to/music");
                eprintln!("  4. Ensure MPD can see your music files");
                e
            })?;
        info!("Path translator initialized successfully");
    }
    Ok(())
}

/// Main entry point for the Muse application.
/// 
/// Initializes logging, parses command-line arguments, and routes commands
/// to the appropriate module functions. All operations return Results for
/// consistent error handling throughout the application.
/// 
/// # Error Handling
/// 
/// Uses `anyhow::Result` for rich error context. Errors are automatically
/// propagated and displayed to the user with helpful context messages.
/// 
/// # Logging
/// 
/// Initializes environment logger which can be controlled via `RUST_LOG`:
/// - `RUST_LOG=debug muse command` - Enable debug logging
/// - `RUST_LOG=muse::algorithm=trace muse play` - Module-specific logging
fn main() -> Result<()> {
    // Initialize environment logger for debugging and monitoring
    env_logger::init();
    
    // Parse command-line arguments using Clap derive macros
    let args = cli::Args::parse();
    
    // Route commands to appropriate module functions
    match args.command {
        cli::Command::InitDb { path, force, no_metadata } => {
            info!("Initializing music database from: {}", path.display());
            db::init_database(&path, force, !no_metadata)?;
        }
        cli::Command::Update { path, scan_depth, remove_missing } => {
            info!("Updating music database from: {}", path.display());
            db::update_database(&path, scan_depth, remove_missing)?;
        }
        cli::Command::List => {
            db::list_songs()?;
        }
        cli::Command::Play { mode } => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            if mode == "algorithm" {
                mpd_client::play("algorithm")?;
            } else if mode == "shuffle" {
                mpd_client::play("shuffle")?;
            } else {
                return Err(anyhow::anyhow!("Unknown play mode: {mode}. Use 'algorithm' or 'shuffle'"));
            }
        }
        cli::Command::Current { song, verbose } => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Generating current queue starting from: {song}");
            let queue_generator = queue::QueueGeneratorV2::new(&config::get_db_path()?.to_string_lossy())?;
            
            if verbose {
                let queue = queue_generator.generate_current_verbose(&song)?;
                mpd_client::load_queue(&queue)?;
            } else {
                let paths = queue_generator.generate_current(&song)?;
                
                // Convert paths to QueuedSong objects
                let queue: Vec<queue::QueuedSong> = paths.into_iter().map(|path| queue::QueuedSong {
                    path: path.clone(),
                    artist: "Unknown".to_string(),
                    title: "Unknown".to_string(),
                    score: 1.0,
                }).collect();
                
                mpd_client::load_queue(&queue)?;
            }
        }
        cli::Command::Thread { song, verbose } => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Generating thread queue starting from: {song}");
            let queue_generator = queue::QueueGeneratorV2::new(&config::get_db_path()?.to_string_lossy())?;
            
            if verbose {
                let queue = queue_generator.generate_thread_verbose(&song)?;
                mpd_client::load_queue(&queue)?;
            } else {
                let paths = queue_generator.generate_thread(&song)?;
                
                // Convert paths to QueuedSong objects
                let queue: Vec<queue::QueuedSong> = paths.into_iter().map(|path| queue::QueuedSong {
                    path: path.clone(),
                    artist: "Unknown".to_string(),
                    title: "Unknown".to_string(),
                    score: 1.0,
                }).collect();
                
                mpd_client::load_queue(&queue)?;
            }
        }
        cli::Command::Stream { song, verbose } => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Generating stream queue starting from: {song}");
            let queue_generator = queue::QueueGeneratorV2::new(&config::get_db_path()?.to_string_lossy())?;
            
            if verbose {
                let queue = queue_generator.generate_stream_verbose(&song)?;
                mpd_client::load_queue(&queue)?;
            } else {
                let paths = queue_generator.generate_stream(&song)?;
                
                // Convert paths to QueuedSong objects
                let queue: Vec<queue::QueuedSong> = paths.into_iter().map(|path| queue::QueuedSong {
                    path: path.clone(),
                    artist: "Unknown".to_string(),
                    title: "Unknown".to_string(),
                    score: 1.0,
                }).collect();
                
                mpd_client::load_queue(&queue)?;
            }
        }
        cli::Command::Completion { shell } => {
            let mut cmd = cli::Args::command();
            completion::generate_completions(completion::shell_to_completion_shell(&shell), &mut cmd);
        }
        cli::Command::CompletionEnhanced { shell } => {
            match shell {
                cli::Shell::Bash => completion::generate_enhanced_bash_completion(),
                cli::Shell::Fish => completion::generate_enhanced_fish_completion(),
                _ => return Err(anyhow::anyhow!("Enhanced completions only supported for bash and fish")),
            }
        }
        cli::Command::CompleteSongs => {
            // This is used by shell completion scripts to get available songs
            completion::print_song_completions()?;
        }
        cli::Command::CompleteSongsFish => {
            // This is used by fish shell completion scripts to get available songs
            completion::print_song_completions_for_shell(Some("fish"))?;
        }
        cli::Command::Next => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Advancing to next track with behavior tracking");
            mpd_client::next_with_tracking()?;
        }
        cli::Command::Skip => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Skipping current track with explicit skip tracking");
            mpd_client::skip_with_tracking()?;
        }
        cli::Command::Love => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Marking current track as loved");
            mpd_client::love_current_track()?;
        }
        cli::Command::Unlove => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            info!("Removing loved status from current track");
            mpd_client::unlove_current_track()?;
        }
        cli::Command::Info => {
            // Initialize path translator for MPD integration
            ensure_path_translator_ready()?;
            
            mpd_client::show_current_song_info()?;
        }
        cli::Command::Daemon { action } => {
            use muse::daemon;
            
            match action {
                cli::DaemonAction::Start => {
                    // Check if daemon is already running
                    if daemon::is_daemon_running()? {
                        eprintln!("Daemon is already running");
                        return Ok(());
                    }
                    
                    // Fork and start daemon in background
                    match unsafe { libc::fork() } {
                        0 => {
                            // Child process - become daemon
                            let mut daemon = daemon::BehaviorDaemon::new()?;
                            daemon.start_monitoring()?;
                            std::process::exit(0);
                        }
                        pid if pid > 0 => {
                            // Parent process
                            println!("Starting behavior tracking daemon...");
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            
                            if daemon::is_daemon_running()? {
                                println!("Daemon started successfully");
                            } else {
                                eprintln!("Failed to start daemon");
                            }
                        }
                        _ => {
                            eprintln!("Failed to fork process");
                        }
                    }
                }
                cli::DaemonAction::Stop => {
                    daemon::stop_daemon()?;
                    println!("Daemon stopped");
                }
                cli::DaemonAction::Status => {
                    if daemon::is_daemon_running()? {
                        println!("Daemon is running");
                    } else {
                        println!("Daemon is not running");
                    }
                }
            }
        }
    }
    
    Ok(())
}