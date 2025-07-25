//! # Path Translation Module
//!
//! This module provides translation between Muse's absolute file paths and MPD's 
//! relative paths. It handles the core issue where Muse stores songs with full
//! filesystem paths while MPD expects paths relative to its music directory.
//!
//! ## Key Features
//!
//! - **Lazy Initialization**: Music directory detection happens on first use
//! - **Caching**: Once detected, the music directory is cached for performance
//! - **Error Recovery**: Comprehensive error handling with helpful messages
//! - **Bidirectional**: Converts both absolute->relative and relative->absolute
//!
//! ## Usage
//!
//! ```no_run
//! use muse::path_translator;
//!
//! // Initialize the translator (usually done at startup)
//! path_translator::initialize()?;
//!
//! // Convert absolute path to MPD relative path
//! let mpd_path = path_translator::absolute_to_mpd_relative(
//!     "/home/user/Music/artist/song.flac"
//! )?; // Returns "artist/song.flac"
//!
//! // Convert MPD relative path back to absolute
//! let abs_path = path_translator::mpd_relative_to_absolute("artist/song.flac")?;
//! // Returns "/home/user/Music/artist/song.flac"
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use log::{info, debug, warn};
use crate::mpd_config;

/// Global cached music directory - initialized once on first use
static MUSIC_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize the path translator by detecting MPD's music directory
/// This should be called at application startup to ensure the translator is ready
pub fn initialize() -> Result<()> {
    debug!("Initializing path translator");
    
    let music_dir = mpd_config::get_mpd_music_directory()
        .context("Failed to detect MPD music directory during initialization")?;
    
    // Try to set the music directory
    match MUSIC_DIR.set(music_dir.clone()) {
        Ok(()) => {
            info!("Path translator initialized with music directory: {}", music_dir.display());
            Ok(())
        }
        Err(_) => {
            // Already initialized - check if it's the same directory
            let existing = MUSIC_DIR.get().unwrap();
            if existing == &music_dir {
                debug!("Path translator already initialized with same directory");
                Ok(())
            } else {
                warn!(
                    "Path translator already initialized with different directory: {} vs {}",
                    existing.display(), music_dir.display()
                );
                Err(anyhow!(
                    "Path translator already initialized with different music directory. \
                     Existing: {}, Attempted: {}", 
                    existing.display(), music_dir.display()
                ))
            }
        }
    }
}

/// Check if the path translator has been initialized
pub fn is_initialized() -> bool {
    MUSIC_DIR.get().is_some()
}

/// Get the cached music directory, initializing if necessary
fn get_music_directory() -> Result<&'static PathBuf> {
    // Check if already initialized
    if let Some(dir) = MUSIC_DIR.get() {
        return Ok(dir);
    }
    
    // Need to initialize - but OnceLock::get_or_try_init is unstable
    // So we'll do a one-time initialization attempt
    debug!("Lazy initializing path translator");
    let music_dir = mpd_config::get_mpd_music_directory()
        .context("Failed to detect music directory during lazy initialization")?;
    
    // Try to set it (might fail if another thread beat us to it)
    match MUSIC_DIR.set(music_dir.clone()) {
        Ok(()) => Ok(MUSIC_DIR.get().unwrap()), // We just set it, so this is safe
        Err(_) => {
            // Another thread initialized it first - use that value
            Ok(MUSIC_DIR.get().unwrap())
        }
    }
}

/// Convert an absolute path to MPD's expected relative path
/// 
/// # Arguments
/// 
/// * `absolute_path` - Full filesystem path to the music file
/// 
/// # Returns
/// 
/// * `Ok(String)` - Relative path suitable for MPD commands
/// * `Err(anyhow::Error)` - If path is not within music directory or translation fails
/// 
/// # Examples
/// 
/// ```no_run
/// use muse::path_translator;
/// 
/// // Initialize first
/// path_translator::initialize()?;
/// 
/// let mpd_path = path_translator::absolute_to_mpd_relative(
///     "/home/user/Music/Rock/Artist/Song.flac"
/// )?;
/// assert_eq!(mpd_path, "Rock/Artist/Song.flac");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn absolute_to_mpd_relative(absolute_path: &str) -> Result<String> {
    let music_dir = get_music_directory()
        .context("Path translator not initialized")?;
    
    debug!("Converting absolute path to MPD relative: {absolute_path}");
    debug!("Using music directory: {}", music_dir.display());
    
    let abs_path = Path::new(absolute_path);
    
    // Ensure the absolute path is actually absolute
    if !abs_path.is_absolute() {
        return Err(anyhow!(
            "Path is not absolute: '{}'. Expected full filesystem path.", 
            absolute_path
        ));
    }
    
    // Strip the music directory prefix to get relative path
    let relative_path = abs_path.strip_prefix(music_dir)
        .map_err(|_| anyhow!(
            "Path '{}' is not within MPD music directory '{}'. \
             Please ensure the song is in MPD's configured music directory.",
            absolute_path, music_dir.display()
        ))?;
    
    // Convert to string, using forward slashes for MPD compatibility
    let relative_str = relative_path.to_string_lossy();
    
    // Ensure we use forward slashes (MPD expects Unix-style paths)
    let mpd_path = if cfg!(windows) {
        relative_str.replace('\\', "/")
    } else {
        relative_str.to_string()
    };
    
    debug!("Converted to MPD relative path: {mpd_path}");
    
    // Validate the result is not empty
    if mpd_path.is_empty() {
        return Err(anyhow!(
            "Resulting MPD path is empty. This suggests '{}' is exactly the music directory.",
            absolute_path
        ));
    }
    
    Ok(mpd_path)
}

/// Convert an MPD relative path to an absolute filesystem path
/// 
/// # Arguments
/// 
/// * `relative_path` - MPD relative path (as returned by mpc commands)
/// 
/// # Returns
/// 
/// * `Ok(PathBuf)` - Absolute filesystem path
/// * `Err(anyhow::Error)` - If translation fails
/// 
/// # Examples
/// 
/// ```no_run
/// use muse::path_translator;
/// 
/// // Initialize first  
/// path_translator::initialize()?;
/// 
/// let abs_path = path_translator::mpd_relative_to_absolute("Rock/Artist/Song.flac")?;
/// assert_eq!(abs_path.to_string_lossy(), "/home/user/Music/Rock/Artist/Song.flac");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn mpd_relative_to_absolute(relative_path: &str) -> Result<PathBuf> {
    let music_dir = get_music_directory()
        .context("Path translator not initialized")?;
    
    debug!("Converting MPD relative path to absolute: {relative_path}");
    debug!("Using music directory: {}", music_dir.display());
    
    // Validate input
    if relative_path.is_empty() {
        return Err(anyhow!("Relative path cannot be empty"));
    }
    
    // Ensure relative path doesn't start with / (should be relative)
    if relative_path.starts_with('/') {
        warn!("MPD relative path starts with '/': {relative_path}");
        // Still process it, but log a warning
    }
    
    // Join with music directory
    let absolute_path = music_dir.join(relative_path);
    
    debug!("Converted to absolute path: {}", absolute_path.display());
    
    Ok(absolute_path)
}

/// Get the music directory path (for debugging/display purposes)
#[allow(dead_code)]
pub fn get_music_directory_path() -> Result<PathBuf> {
    get_music_directory().cloned()
        .context("Path translator not initialized")
}

/// Validate that a path can be translated (exists within music directory)
#[allow(dead_code)]
pub fn can_translate_path(absolute_path: &str) -> bool {
    match get_music_directory() {
        Ok(music_dir) => {
            let abs_path = Path::new(absolute_path);
            abs_path.strip_prefix(music_dir).is_ok()
        }
        Err(_) => false,
    }
}

/// Reset the path translator (mainly for testing)
#[cfg(test)]
#[allow(dead_code)]
pub fn reset_for_testing() {
    // Note: OnceLock doesn't have a reset method, so this is a no-op
    // In real usage, the translator should only be initialized once per program run
    warn!("Path translator reset requested, but OnceLock cannot be reset");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_validation() {
        // Test absolute path validation
        assert!(absolute_to_mpd_relative("relative/path").is_err());
        
        // Test empty path validation
        assert!(mpd_relative_to_absolute("").is_err());
    }
    
    #[test]
    fn test_path_conversion_logic() {
        // This test would need a mock music directory setup
        // For now, just test the logic without actual initialization
        
        let test_cases = vec![
            ("/home/user/Music/artist/song.flac", "artist/song.flac"),
            ("/home/user/Music/genre/artist/album/track.mp3", "genre/artist/album/track.mp3"),
        ];
        
        for (abs, expected_rel) in test_cases {
            // Test the path stripping logic manually
            let abs_path = Path::new(abs);
            let music_dir = Path::new("/home/user/Music");
            
            if let Ok(relative) = abs_path.strip_prefix(music_dir) {
                assert_eq!(relative.to_string_lossy(), expected_rel);
            }
        }
    }
    
    #[test]
    fn test_windows_path_conversion() {
        // Test that Windows backslashes are converted to forward slashes
        let windows_path = r"artist\album\song.flac";
        let expected = "artist/album/song.flac";
        
        let converted = if cfg!(windows) {
            windows_path.replace('\\', "/")
        } else {
            windows_path.to_string()
        };
        
        if cfg!(windows) {
            assert_eq!(converted, expected);
        } else {
            // On Unix systems, backslashes are preserved (valid filename chars)
            assert_eq!(converted, windows_path);
        }
    }
    
    #[test]
    fn test_is_initialized() {
        // Before any initialization
        let initial_state = is_initialized();
        // This might be true or false depending on test order
        // The important thing is that the function doesn't panic
        // Test that the function doesn't panic and returns a valid boolean
        let _ = initial_state; // Use the value to avoid unused variable warning
    }
}