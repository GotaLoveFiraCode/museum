//! # MPD Configuration Detection Module
//!
//! This module handles detection of MPD's music directory through multiple strategies:
//! - Parsing MPD configuration files
//! - Cross-referencing MPD database with Muse database
//! - Statistical analysis of existing song paths
//! - Common directory fallbacks
//!
//! The goal is to determine the base directory that MPD uses for relative paths
//! so we can convert Muse's absolute paths to MPD's expected relative paths.

use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use log::{info, warn, debug};
use crate::db;

/// Detect MPD's music directory using multiple fallback strategies
pub fn get_mpd_music_directory() -> Result<PathBuf> {
    info!("Detecting MPD music directory...");
    
    // Strategy 1: Parse MPD config file
    if let Ok(dir) = parse_mpd_config_file() {
        info!("Found MPD music directory from config: {}", dir.display());
        return Ok(dir);
    }
    
    // Strategy 2: Cross-reference MPD and Muse databases
    if let Ok(dir) = detect_from_databases() {
        info!("Detected MPD music directory from database comparison: {}", dir.display());
        return Ok(dir);
    }
    
    // Strategy 3: Statistical analysis of Muse database paths
    if let Ok(dir) = infer_from_song_database() {
        info!("Inferred MPD music directory from song paths: {}", dir.display());
        return Ok(dir);
    }
    
    // Strategy 4: Common defaults
    if let Ok(dir) = check_common_directories() {
        info!("Using common default MPD music directory: {}", dir.display());
        return Ok(dir);
    }
    
    Err(anyhow!(
        "Could not determine MPD music directory. Please ensure:\n\
         1. MPD is running and accessible\n\
         2. MPD configuration is valid\n\
         3. Music database is not empty\n\
         4. Music files are in a standard location"
    ))
}

/// Parse MPD configuration file to find music_directory setting
fn parse_mpd_config_file() -> Result<PathBuf> {
    debug!("Attempting to parse MPD config file");
    
    // Try common MPD config locations
    let config_paths = [
        dirs::config_dir().map(|p| p.join("mpd").join("mpd.conf")),
        dirs::home_dir().map(|p| p.join(".config").join("mpd").join("mpd.conf")),
        dirs::home_dir().map(|p| p.join(".mpdconf")),
        Some(PathBuf::from("/etc/mpd.conf")),
        Some(PathBuf::from("/usr/local/etc/mpd.conf")),
    ];
    
    for maybe_path in config_paths.iter().flatten() {
        if let Ok(music_dir) = parse_config_file(maybe_path) {
            return Ok(music_dir);
        }
    }
    
    Err(anyhow!("No valid MPD config file found"))
}

/// Parse a specific MPD config file for music_directory setting
fn parse_config_file(config_path: &Path) -> Result<PathBuf> {
    debug!("Parsing MPD config: {}", config_path.display());
    
    if !config_path.exists() {
        return Err(anyhow!("Config file does not exist: {}", config_path.display()));
    }
    
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read MPD config: {}", config_path.display()))?;
    
    // Look for music_directory setting
    for line in content.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        
        // Look for music_directory "path"
        if line.starts_with("music_directory") {
            if let Some(path_part) = line.split_whitespace().nth(1) {
                // Remove quotes if present
                let path_str = path_part.trim_matches('"').trim_matches('\'');
                
                // Expand ~ to home directory
                let expanded_path = if path_str.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        home.join(path_str.strip_prefix("~/").unwrap_or(path_str))
                    } else {
                        PathBuf::from(path_str)
                    }
                } else {
                    PathBuf::from(path_str)
                };
                
                // Verify directory exists
                if expanded_path.exists() && expanded_path.is_dir() {
                    debug!("Found music_directory in config: {}", expanded_path.display());
                    return Ok(expanded_path);
                } else {
                    warn!("Music directory in config does not exist: {}", expanded_path.display());
                }
            }
        }
    }
    
    Err(anyhow!("No valid music_directory found in config"))
}

/// Detect music directory by comparing MPD and Muse databases
fn detect_from_databases() -> Result<PathBuf> {
    debug!("Attempting to detect music directory from database comparison");
    
    // Get first song from MPD
    let mpd_song = get_first_mpd_song()
        .context("Failed to get first song from MPD database")?;
    
    // Find matching song in Muse database
    let muse_song = find_matching_muse_song(&mpd_song)
        .context("Failed to find matching song in Muse database")?;
    
    // Extract music directory by comparing paths
    extract_music_directory(&muse_song, &mpd_song)
}

/// Get the first song from MPD's database
fn get_first_mpd_song() -> Result<String> {
    debug!("Getting first song from MPD database");
    
    let output = Command::new("mpc")
        .args(["listall"])
        .output()
        .context("Failed to execute 'mpc listall'")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("mpc listall failed: {}", stderr.trim()));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()
        .ok_or_else(|| anyhow!("MPD database is empty"))?;
    
    Ok(first_line.trim().to_string())
}

/// Find a Muse song that matches the MPD song by filename
fn find_matching_muse_song(mpd_relative_path: &str) -> Result<String> {
    debug!("Finding matching Muse song for: {mpd_relative_path}");
    
    // Extract filename from MPD path
    let mpd_filename = Path::new(mpd_relative_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Could not extract filename from MPD path"))?;
    
    // Search Muse database for songs with matching filename
    let conn = db::get_connection()
        .context("Failed to connect to Muse database")?;
    
    let mut stmt = conn.prepare("SELECT path FROM songs WHERE path LIKE ?")
        .context("Failed to prepare database query")?;
    
    let pattern = format!("%{mpd_filename}");
    let mut rows = stmt.query_map([&pattern], |row| {
        row.get::<_, String>(0)
    }).context("Failed to execute database query")?;
    
    if let Some(row) = rows.next() {
        let path = row.context("Failed to read database row")?;
        debug!("Found matching Muse song: {path}");
        return Ok(path);
    }
    
    Err(anyhow!("No matching song found in Muse database for: {}", mpd_filename))
}

/// Extract music directory by comparing absolute and relative paths
fn extract_music_directory(muse_absolute_path: &str, mpd_relative_path: &str) -> Result<PathBuf> {
    debug!("Extracting music directory from paths:");
    debug!("  Muse absolute: {muse_absolute_path}");
    debug!("  MPD relative: {mpd_relative_path}");
    
    let absolute_path = Path::new(muse_absolute_path);
    let relative_path = Path::new(mpd_relative_path);
    
    // Find the directory that when joined with relative gives absolute
    // We need to find the longest path prefix that when removed from absolute_path gives relative_path
    let abs_components: Vec<_> = absolute_path.components().collect();
    let rel_components: Vec<_> = relative_path.components().collect();
    
    // The music directory should be the prefix of absolute_path that, when removed, leaves relative_path
    if abs_components.len() >= rel_components.len() {
        let music_dir_len = abs_components.len() - rel_components.len();
        let mut music_dir = PathBuf::new();
        
        for component in &abs_components[..music_dir_len] {
            music_dir.push(component);
        }
        
        // Verify this gives us the correct relative path
        if let Ok(remaining) = absolute_path.strip_prefix(&music_dir) {
            if remaining == relative_path {
                debug!("Extracted music directory: {}", music_dir.display());
                return Ok(music_dir);
            }
        }
    }
    
    Err(anyhow!("Could not extract music directory from path comparison"))
}

/// Infer music directory from statistical analysis of song paths
fn infer_from_song_database() -> Result<PathBuf> {
    debug!("Inferring music directory from song database");
    
    let conn = db::get_connection()
        .context("Failed to connect to database")?;
    
    let mut stmt = conn.prepare("SELECT path FROM songs LIMIT 100")
        .context("Failed to prepare query")?;
    
    let paths: Vec<String> = stmt.query_map([], |row| {
        row.get::<_, String>(0)
    })?
    .collect::<Result<Vec<_>, _>>()
    .context("Failed to collect song paths")?;
    
    if paths.is_empty() {
        return Err(anyhow!("No songs in database"));
    }
    
    // Find longest common prefix
    let common_prefix = find_longest_common_prefix(&paths);
    
    if let Some(prefix_path) = common_prefix {
        let prefix_dir = if prefix_path.is_dir() {
            prefix_path
        } else {
            prefix_path.parent()
                .ok_or_else(|| anyhow!("Could not get parent directory"))?
                .to_path_buf()
        };
        
        debug!("Inferred music directory from common prefix: {}", prefix_dir.display());
        return Ok(prefix_dir);
    }
    
    Err(anyhow!("Could not infer music directory from song paths"))
}

/// Find longest common prefix of all paths
fn find_longest_common_prefix(paths: &[String]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }
    
    let first_path = Path::new(&paths[0]);
    let mut common_components = Vec::new();
    
    for component in first_path.components() {
        let component_str = component.as_os_str();
        
        // Check if all paths have this component at this position
        let all_have_component = paths.iter().all(|path| {
            let path_components: Vec<_> = Path::new(path).components().collect();
            path_components.len() > common_components.len() &&
            path_components[common_components.len()].as_os_str() == component_str
        });
        
        if all_have_component {
            common_components.push(component);
        } else {
            break;
        }
    }
    
    if !common_components.is_empty() {
        let mut result = PathBuf::new();
        for component in common_components {
            result.push(component);
        }
        Some(result)
    } else {
        None
    }
}

/// Check common default directories for music
fn check_common_directories() -> Result<PathBuf> {
    debug!("Checking common music directories");
    
    let common_dirs = [
        dirs::home_dir().map(|p| p.join("Music")),
        dirs::home_dir().map(|p| p.join("music")),
        Some(PathBuf::from("/var/lib/mpd/music")),
        Some(PathBuf::from("/usr/share/music")),
        Some(PathBuf::from("/opt/music")),
    ];
    
    for maybe_dir in common_dirs.iter().flatten() {
        if maybe_dir.exists() && maybe_dir.is_dir() {
            // Check if this directory has music files
            if has_music_files(maybe_dir)? {
                debug!("Found common music directory: {}", maybe_dir.display());
                return Ok(maybe_dir.clone());
            }
        }
    }
    
    Err(anyhow!("No common music directories found"))
}

/// Check if a directory contains music files
fn has_music_files(dir: &Path) -> Result<bool> {
    use std::fs::read_dir;
    
    let entries = read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    
    for entry in entries.take(10) { // Check first 10 entries
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "flac" | "mp3" | "ogg" | "m4a" | "wav") {
                    return Ok(true);
                }
            }
        } else if path.is_dir() {
            // Recursively check subdirectories (limited depth)
            if has_music_files(&path)? {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}