//! # Configuration Module
//!
//! This module handles configuration management and data directory setup for Muse.
//! It provides platform-appropriate data storage locations and ensures necessary
//! directories exist.
//!
//! ## Data Storage
//!
//! Muse stores its database in the platform-standard data directory:
//! - Linux: `~/.local/share/muse/`
//! - macOS: `~/Library/Application Support/muse/`
//! - Windows: `%APPDATA%\muse\`
//!
//! ## Future Configuration
//!
//! This module is designed to be extended with additional configuration options:
//! - User preferences (algorithm tuning parameters)
//! - MPD connection settings
//! - Custom data directory locations
//! - Import/export settings

use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};

/// Returns the platform-appropriate database file path.
/// 
/// This function locates the standard data directory for the current platform
/// and creates the Muse subdirectory if it doesn't exist. The database file
/// is named `music.db` and stores all song metadata and learning data.
/// 
/// # Platform Behavior
/// 
/// - **Linux**: `~/.local/share/muse/music.db`
/// - **macOS**: `~/Library/Application Support/muse/music.db`
/// - **Windows**: `%APPDATA%\muse\music.db`
/// 
/// # Directory Creation
/// 
/// The function automatically creates the `muse` subdirectory if it doesn't
/// exist, ensuring the database can be created successfully.
/// 
/// # Returns
/// 
/// * `Ok(PathBuf)` - Path to the database file
/// * `Err(anyhow::Error)` - If data directory cannot be determined or created
/// 
/// # Errors
/// 
/// This function will return an error if:
/// - The system data directory cannot be determined
/// - The muse subdirectory cannot be created due to permissions
/// - The filesystem is read-only
/// 
/// # Examples
/// 
/// ```no_run
/// use muse::config::get_db_path;
/// 
/// let db_path = get_db_path()?;
/// println!("Database location: {}", db_path.display());
/// # Ok::<(), anyhow::Error>(())
/// ```
/// 
/// # Design Notes
/// 
/// Using the standard data directory ensures:
/// - Proper separation from user documents
/// - Automatic cleanup by system maintenance tools
/// - Compliance with platform conventions
/// - Backup software typically includes these directories
pub fn get_db_path() -> Result<PathBuf> {
    // Get platform-appropriate data directory
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!(
            "Could not determine system data directory. Please ensure your platform supports standard data directories."
        ))?;
    
    // Create muse subdirectory
    let muse_dir = data_dir.join("muse");
    fs::create_dir_all(&muse_dir)
        .with_context(|| format!(
            "Failed to create Muse data directory at {}. Please check file permissions.",
            muse_dir.display()
        ))?;
    
    // Return path to database file
    Ok(muse_dir.join("music.db"))
}

/// Returns the platform-appropriate data directory for Muse
/// 
/// This function is similar to `get_db_path` but returns the directory itself
/// rather than the database file path. Used for storing other application data
/// like daemon PID files.
/// 
/// # Returns
/// 
/// * `Ok(PathBuf)` - Path to the muse data directory
/// * `Err(anyhow::Error)` - If data directory cannot be determined or created
#[allow(dead_code)]
pub fn get_data_dir() -> Result<PathBuf> {
    // Get platform-appropriate data directory
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!(
            "Could not determine system data directory. Please ensure your platform supports standard data directories."
        ))?;
    
    // Create muse subdirectory
    let muse_dir = data_dir.join("muse");
    fs::create_dir_all(&muse_dir)
        .with_context(|| format!(
            "Failed to create Muse data directory at {}. Please check file permissions.",
            muse_dir.display()
        ))?;
    
    Ok(muse_dir)
}

/// Configuration for runtime behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Path to the database file
    pub db_path: PathBuf,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            db_path: get_db_path().unwrap_or_else(|_| PathBuf::from("music.db")),
        }
    }
}

#[allow(dead_code)]
impl RuntimeConfig {
    /// Create a new runtime configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            db_path: get_db_path()?,
        })
    }

    /// Create configuration with explicit database path
    pub fn with_db_path(db_path: PathBuf) -> Self {
        Self {
            db_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_db_path_returns_valid_path() {
        let result = get_db_path();
        assert!(result.is_ok());
        
        let path = result.unwrap();
        assert!(path.file_name().is_some());
        assert_eq!(path.file_name().unwrap(), "music.db");
        assert!(path.parent().is_some());
    }

    #[test]
    fn test_get_db_path_creates_directory() {
        // This test verifies the function creates directories
        // but doesn't interfere with the actual user directory
        let result = get_db_path();
        assert!(result.is_ok());
        
        let path = result.unwrap();
        let parent_dir = path.parent().expect("Database path should have parent");
        
        // Directory should exist after calling get_db_path
        assert!(parent_dir.exists());
        assert!(parent_dir.is_dir());
    }

    #[test]
    fn test_get_db_path_consistent_results() {
        // Multiple calls should return the same path
        let path1 = get_db_path().expect("First call should succeed");
        let path2 = get_db_path().expect("Second call should succeed");
        
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_db_path_structure() {
        let path = get_db_path().expect("Should get valid path");
        
        // Path should end with muse/music.db
        assert!(path.to_string_lossy().contains("muse"));
        assert!(path.to_string_lossy().ends_with("music.db"));
        
        // Parent directory should be named "muse"
        let parent = path.parent().expect("Should have parent directory");
        assert_eq!(parent.file_name().unwrap(), "muse");
    }

    #[test]
    fn test_db_path_absolute() {
        let path = get_db_path().expect("Should get valid path");
        assert!(path.is_absolute(), "Database path should be absolute");
    }
}