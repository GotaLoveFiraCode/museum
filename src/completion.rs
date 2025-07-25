//! # Shell Completion Module
//!
//! This module provides shell completion functionality for Muse, including:
//! - Generation of completion scripts for various shells
//! - Custom completion for song names from the database
//! - Integration with clap's completion system
//!
//! ## Usage
//!
//! ```bash
//! # Generate bash completions
//! muse completion bash > ~/.local/share/bash-completion/completions/muse
//!
//! # Generate zsh completions  
//! muse completion zsh > ~/.config/zsh/completions/_muse
//! ```

use crate::db;
use crate::config;
use anyhow::Result;
use clap::Command;
use clap_complete::{generate, Generator, Shell as CompletionShell};
use std::io;

/// Generate shell completions for the given shell
pub fn generate_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

/// Generate enhanced fish completion script with song name completion
pub fn generate_enhanced_fish_completion() {
    println!(r#"# Enhanced Muse completion script for Fish shell with song name completion
# Install with: muse completion-enhanced fish > ~/.config/fish/completions/muse.fish

# Function to get song completions
function __muse_complete_songs
    # Get song completions from muse command, suppress errors
    if command -sq muse
        muse complete-songs-fish 2>/dev/null
    end
end

# Function to get directory completions (for update command)
function __muse_complete_directories
    # Use fish's built-in directory completion
    __fish_complete_directories
end

# Clear existing completions to avoid conflicts
complete -c muse -e

# Global options
complete -c muse -s h -l help -d 'Print help information'
complete -c muse -s V -l version -d 'Print version information'

# Main commands
complete -c muse -f -n '__fish_is_first_token' -a 'init-db' -d 'Initialize database from music directory (full scan)'
complete -c muse -f -n '__fish_is_first_token' -a 'update' -d 'Update database with new files (incremental)'
complete -c muse -f -n '__fish_is_first_token' -a 'list' -d 'List all songs in the database'
complete -c muse -f -n '__fish_is_first_token' -a 'play' -d 'Play music using MPD'
complete -c muse -f -n '__fish_is_first_token' -a 'current' -d 'Generate dual-path queue based on starting song'
complete -c muse -f -n '__fish_is_first_token' -a 'thread' -d 'Generate single-path queue based on starting song'
complete -c muse -f -n '__fish_is_first_token' -a 'stream' -d 'Generate training queue with randomness'
complete -c muse -f -n '__fish_is_first_token' -a 'next' -d 'Skip to next track with behavior tracking'
complete -c muse -f -n '__fish_is_first_token' -a 'skip' -d 'Skip current track with explicit skip tracking'
complete -c muse -f -n '__fish_is_first_token' -a 'love' -d 'Mark current track as loved'
complete -c muse -f -n '__fish_is_first_token' -a 'unlove' -d 'Remove loved status from current track'
complete -c muse -f -n '__fish_is_first_token' -a 'info' -d 'Show detailed information about the current song'
complete -c muse -f -n '__fish_is_first_token' -a 'daemon' -d 'Manage the behavior tracking daemon'
complete -c muse -f -n '__fish_is_first_token' -a 'completion' -d 'Generate shell completions'
complete -c muse -f -n '__fish_is_first_token' -a 'completion-enhanced' -d 'Generate enhanced shell completions'
complete -c muse -f -n '__fish_is_first_token' -a 'help' -d 'Print help for commands'

# init-db command - complete with directories and options
complete -c muse -n '__fish_seen_subcommand_from init-db' -a '(__muse_complete_directories)' -d 'Music directory path'
complete -c muse -f -n '__fish_seen_subcommand_from init-db' -l force -d 'Force overwrite existing database'
complete -c muse -f -n '__fish_seen_subcommand_from init-db' -l no-metadata -d 'Skip metadata extraction (paths only)'

# update command - complete with directories and options
complete -c muse -n '__fish_seen_subcommand_from update' -a '(__muse_complete_directories)' -d 'Music directory path'
complete -c muse -f -n '__fish_seen_subcommand_from update' -l scan-depth -d 'Maximum scan depth' -r
complete -c muse -f -n '__fish_seen_subcommand_from update' -l remove-missing -d 'Remove entries for missing files'

# play command - complete with play modes
complete -c muse -f -n '__fish_seen_subcommand_from play' -a 'algorithm' -d 'Use intelligent scoring system'
complete -c muse -f -n '__fish_seen_subcommand_from play' -a 'shuffle' -d 'Use random playback'

# current, thread, stream commands - complete with song names and options
complete -c muse -n '__fish_seen_subcommand_from current' -a '(__muse_complete_songs)' -d 'Song name, artist, or album'
complete -c muse -f -n '__fish_seen_subcommand_from current' -s v -l verbose -d 'Enable verbose output showing algorithm decisions'
complete -c muse -n '__fish_seen_subcommand_from thread' -a '(__muse_complete_songs)' -d 'Song name, artist, or album'
complete -c muse -f -n '__fish_seen_subcommand_from thread' -s v -l verbose -d 'Enable verbose output showing algorithm decisions'
complete -c muse -n '__fish_seen_subcommand_from stream' -a '(__muse_complete_songs)' -d 'Song name, artist, or album'
complete -c muse -f -n '__fish_seen_subcommand_from stream' -s v -l verbose -d 'Enable verbose output showing algorithm decisions'

# daemon command - complete with daemon actions
complete -c muse -f -n '__fish_seen_subcommand_from daemon' -a 'start' -d 'Start the behavior tracking daemon'
complete -c muse -f -n '__fish_seen_subcommand_from daemon' -a 'stop' -d 'Stop the running daemon'
complete -c muse -f -n '__fish_seen_subcommand_from daemon' -a 'status' -d 'Check daemon status'

# completion command - complete with shell types
complete -c muse -f -n '__fish_seen_subcommand_from completion' -a 'bash' -d 'Generate bash completions'
complete -c muse -f -n '__fish_seen_subcommand_from completion' -a 'zsh' -d 'Generate zsh completions'
complete -c muse -f -n '__fish_seen_subcommand_from completion' -a 'fish' -d 'Generate fish completions'
complete -c muse -f -n '__fish_seen_subcommand_from completion' -a 'power-shell' -d 'Generate PowerShell completions'
complete -c muse -f -n '__fish_seen_subcommand_from completion' -a 'elvish' -d 'Generate elvish completions'

# completion-enhanced command - complete with shell types (currently supports bash and fish)
complete -c muse -f -n '__fish_seen_subcommand_from completion-enhanced' -a 'bash' -d 'Generate enhanced bash completions'
complete -c muse -f -n '__fish_seen_subcommand_from completion-enhanced' -a 'fish' -d 'Generate enhanced fish completions'

# help command - complete with subcommands for help topics
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'init-db' -d 'Help for init-db command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'update' -d 'Help for update command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'list' -d 'Help for list command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'play' -d 'Help for play command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'current' -d 'Help for current command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'thread' -d 'Help for thread command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'stream' -d 'Help for stream command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'next' -d 'Help for next command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'skip' -d 'Help for skip command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'love' -d 'Help for love command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'unlove' -d 'Help for unlove command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'info' -d 'Help for info command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'daemon' -d 'Help for daemon command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'completion' -d 'Help for completion command'
complete -c muse -f -n '__fish_seen_subcommand_from help' -a 'completion-enhanced' -d 'Help for completion-enhanced command'
"#);
}

/// Generate enhanced bash completion script with song name completion
pub fn generate_enhanced_bash_completion() {
    println!(r#"#!/bin/bash
# Enhanced Muse completion script with song name completion
# Install with: muse completion-enhanced bash > ~/.local/share/bash-completion/completions/muse

_muse_complete_songs() {{
    # Get song completions from muse command
    local songs
    if command -v muse >/dev/null 2>&1; then
        # Use complete-songs command to get available songs
        mapfile -t songs < <(muse complete-songs 2>/dev/null)
        printf '%s\n' "${{songs[@]}}"
    fi
}}

_muse() {{
    local cur prev words cword
    _init_completion || return

    case "${{prev}}" in
        current|thread|stream)
            # Complete with song names for these commands
            mapfile -t COMPREPLY < <(_muse_complete_songs | grep -i "^${{cur}}")
            return 0
            ;;
        completion|completion-enhanced)
            # Complete with shell types
            COMPREPLY=($(compgen -W "bash zsh fish power-shell elvish" -- "${{cur}}"))
            return 0
            ;;
        daemon)
            # Complete with daemon actions
            COMPREPLY=($(compgen -W "start stop status" -- "${{cur}}"))
            return 0
            ;;
        init-db|update)
            # Complete with directories
            _filedir -d
            return 0
            ;;
        --scan-depth)
            # Complete with numbers for scan depth
            COMPREPLY=($(compgen -W "1 2 3 5 10 15 20" -- "${{cur}}"))
            return 0
            ;;
    esac

    # Check if we're completing a subcommand
    local subcommands="init-db update list play current thread stream next skip love unlove info daemon completion completion-enhanced help"
    
    if [[ $cword -eq 1 ]]; then
        # Complete main commands
        COMPREPLY=($(compgen -W "$subcommands --help --version" -- "${{cur}}"))
    elif [[ $cword -eq 2 ]] && [[ "${{words[1]}}" == "play" ]]; then
        # Complete play modes
        COMPREPLY=($(compgen -W "algorithm shuffle" -- "${{cur}}"))
    else
        # Handle command-specific options
        case "${{words[1]}}" in
            init-db)
                COMPREPLY=($(compgen -W "--force --no-metadata --help" -- "${{cur}}"))
                ;;
            update)
                COMPREPLY=($(compgen -W "--scan-depth --remove-missing --help" -- "${{cur}}"))
                ;;
            current|thread|stream)
                COMPREPLY=($(compgen -W "--verbose -v --help" -- "${{cur}}"))
                ;;
            daemon)
                COMPREPLY=($(compgen -W "start stop status --help" -- "${{cur}}"))
                ;;
            completion|completion-enhanced)
                COMPREPLY=($(compgen -W "bash zsh fish power-shell elvish" -- "${{cur}}"))
                ;;
            *)
                # Default completion
                COMPREPLY=($(compgen -W "$subcommands" -- "${{cur}}"))
                ;;
        esac
    fi
}} &&
complete -F _muse muse

# ex: filetype=sh
"#);
}

/// Convert our Shell enum to clap_complete's Shell enum
pub fn shell_to_completion_shell(shell: &crate::cli::Shell) -> CompletionShell {
    match shell {
        crate::cli::Shell::Bash => CompletionShell::Bash,
        crate::cli::Shell::Zsh => CompletionShell::Zsh,
        crate::cli::Shell::Fish => CompletionShell::Fish,
        crate::cli::Shell::PowerShell => CompletionShell::PowerShell,
        crate::cli::Shell::Elvish => CompletionShell::Elvish,
    }
}

/// Get available song names for completion
/// Returns a list of song titles, artist names, and album names that can be used
/// for completion in the current, thread, and stream commands
pub fn get_song_completions() -> Result<Vec<String>> {
    let db_path = match config::get_db_path() {
        Ok(path) => path,
        Err(_) => return Ok(Vec::new()), // Return empty if no database
    };

    if !db_path.exists() {
        return Ok(Vec::new()); // Return empty if database doesn't exist
    }

    let db_path_str = db_path.to_string_lossy().to_string();
    
    // Try to get songs from the database
    match db::get_all_songs_for_completion(&db_path_str) {
        Ok(songs) => {
            let mut completions = Vec::new();
            
            for song in songs {
                // Add song title
                if !song.title.is_empty() {
                    completions.push(song.title.clone());
                }
                
                // Add artist name (avoid duplicates)
                if !song.artist.is_empty() && !completions.contains(&song.artist) {
                    completions.push(song.artist.clone());
                }
                
                // Add album name (avoid duplicates)  
                if !song.album.is_empty() && !completions.contains(&song.album) {
                    completions.push(song.album.clone());
                }
                
                // Add combined artist - title format for better matching
                if !song.artist.is_empty() && !song.title.is_empty() {
                    let combined = format!("{} - {}", song.artist, song.title);
                    completions.push(combined);
                }
            }
            
            // Sort for consistent output
            completions.sort();
            Ok(completions)
        },
        Err(_) => Ok(Vec::new()), // Return empty on any error
    }
}

/// Print available completions for song names
/// This is used by shell completion systems to get dynamic completions
pub fn print_song_completions() -> Result<()> {
    print_song_completions_for_shell(None)
}

/// Print available completions for song names, formatted for a specific shell
/// This is used by shell completion systems to get dynamic completions
pub fn print_song_completions_for_shell(shell: Option<&str>) -> Result<()> {
    let completions = get_song_completions()?;
    
    for completion in completions {
        match shell {
            Some("fish") => {
                // Fish handles escaping automatically, don't add quotes
                println!("{completion}");
            }
            _ => {
                // For bash, zsh, and other shells, escape spaces and special characters
                if completion.contains(' ') || completion.contains('\t') || completion.contains('\n') {
                    println!("\"{}\"", completion.replace('"', "\\\""));
                } else {
                    println!("{completion}");
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_conversion() {
        assert_eq!(
            shell_to_completion_shell(&crate::cli::Shell::Bash),
            CompletionShell::Bash
        );
        assert_eq!(
            shell_to_completion_shell(&crate::cli::Shell::Zsh),
            CompletionShell::Zsh
        );
    }

    #[test]
    fn test_get_song_completions_empty_db() {
        // This should not panic even if database doesn't exist
        let result = get_song_completions();
        assert!(result.is_ok());
    }
}