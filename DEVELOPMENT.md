# Muse v2 Development Guide

This document provides comprehensive information for developers working on Muse v2.

## 🏗️ Architecture Overview

### Design Philosophy

Muse v2 follows these core principles:

1. **Modularity**: Each module has a single, well-defined responsibility
2. **Performance**: Minimal dependencies and optimized data structures
3. **Clarity**: Code should be self-documenting and easy to understand
4. **Reliability**: Robust error handling and graceful degradation

### Project Structure

```
src/
├── main.rs           # Entry point and command orchestration
├── cli.rs            # Command-line interface definitions
├── db.rs             # Database operations and schema management
├── algorithm.rs      # Scoring algorithms (simple and complex)
├── mpd_client.rs     # MPD integration via mpc command-line tool
├── queue.rs          # Queue generation logic (Current, Thread, Stream)
└── config.rs         # Configuration and data directory management
```

## 📋 Module Responsibilities

### main.rs
**Purpose**: Application entry point and command routing

**Key Functions**:
- Parse command-line arguments
- Initialize logging
- Route commands to appropriate modules
- Handle top-level error reporting

**Dependencies**: All other modules

### cli.rs
**Purpose**: Command-line interface definition using Clap

**Key Structures**:
- `Args`: Main argument parser
- `Command`: Subcommand enumeration

**Features**:
- Type-safe argument parsing
- Built-in help generation
- Validation of required parameters

### db.rs
**Purpose**: SQLite database operations and schema management

**Key Structures**:
- `Song`: Core song data structure
- `SongConnection`: Connection between songs (unused in current implementation)

**Key Functions**:
- `get_connection()`: Database connection with schema initialization
- `update_database()`: Scan filesystem and add new songs
- `list_songs()`: Display all catalogued songs
- `get_song_by_name()`: Search songs by title or artist
- `get_song_connections()`: Retrieve connection data for algorithms

**Schema**:
```sql
songs: id, path, artist, album, title, touches, listens, skips, loved
connections: from_song_id, to_song_id, count
```

### algorithm.rs
**Purpose**: Implementation of both simple and complex scoring algorithms

**Core Algorithm**:
- Songs with < 30 touches use weighted scoring favoring new content
- Songs with ≥ 30 touches use logarithmic dampening for stability
- Connection weighting multiplies base scores by relationship strength

**Key Functions**:
- `calculate_score()`: Main scoring function implementing PDF algorithm
- `weight()`: Dynamic weighting for touch thresholds
- `dampen()`: Logarithmic dampening for established songs
- `apply_connection_weight()`: Apply relationship bonuses

### mpd_client.rs
**Purpose**: MPD integration using mpc command-line tool

**Key Functions**:
- `get_client()`: Verify MPD/mpc availability
- `play()`: Start playback in shuffle or algorithm mode
- `load_queue()`: Load generated queues into MPD
- `play_with_algorithm()`: Score all songs and load top 50

**Design Decision**: Uses `mpc` command-line tool instead of direct MPD protocol for simplicity and reliability.

### queue.rs
**Purpose**: Generate the three queue types (Current, Thread, Stream)

**Queue Types**:
1. **Current**: Dual-path interleaved queue (9-27 songs)
2. **Thread**: Single-path queue (9-27 songs)
3. **Stream**: Training queue with randomness (exactly 30 songs)

**Key Functions**:
- `generate_current()`: Creates mixed dual-path queue
- `generate_thread()`: Creates single-path queue
- `generate_stream()`: Creates training queue with exploration
- `generate_path()`: Helper for following connection chains

### config.rs
**Purpose**: Configuration and data directory management

**Key Functions**:
- `get_db_path()`: Platform-appropriate database location

## 🔧 Development Environment

### Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# MPD for testing
sudo apt install mpd mpc  # Ubuntu/Debian
brew install mpd mpc      # macOS

# Development tools (optional)
cargo install cargo-watch
cargo install cargo-audit
```

### Build System

```bash
# Development builds
cargo check          # Fast compile check
cargo build           # Debug build
cargo build --release # Optimized release build

# Testing
cargo test            # Run tests (when implemented)
cargo clippy          # Linting
cargo fmt             # Formatting

# Continuous development
cargo watch -x check  # Auto-recompile on changes
```

### Release Configuration

```toml
[profile.release]
lto = true           # Link-time optimization
opt-level = 3        # Maximum optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip debug symbols
```

## 📊 Data Flow

### Song Discovery Process

1. **Filesystem Scan**: `db::update_database()` walks directory tree
2. **Metadata Extraction**: Parse filenames for artist/title (currently simplified)
3. **Database Storage**: Insert new songs with zero statistics
4. **Indexing**: Automatic SQLite indexing on path and connections

### Algorithm Learning Process

1. **Song Suggestion**: Algorithm calculates scores for all songs
2. **Playback**: MPD plays suggested song
3. **User Interaction**: User listens completely or skips
4. **Statistics Update**: `touches`, `listens`, or `skips` incremented
5. **Connection Recording**: If song completes, connection to next song recorded

### Queue Generation Process

1. **Starting Song**: User specifies song name for queue base
2. **Song Lookup**: Database search by title/artist (fuzzy matching)
3. **Connection Analysis**: Retrieve all songs connected to starting song
4. **Score Calculation**: Apply both simple and complex algorithms
5. **Path Generation**: Follow highest-scoring connections
6. **Queue Assembly**: Combine paths according to queue type
7. **MPD Loading**: Clear current queue and load generated songs

## 🔍 Algorithm Details

### Simple Algorithm Implementation

```rust
fn calculate_score(song: &Song) -> f64 {
    let mut score = if song.touches < 30 {
        let (weight_listens, weight_skips) = weight(song.touches);
        (weight_listens as f64 * song.listens as f64) - 
        (weight_skips as f64 * song.skips as f64)
    } else {
        let dampening = dampen(song.touches);
        dampening * song.listens as f64 - dampening * song.skips as f64
    };
    
    if score < 0.0 { score = 0.0; }
    if song.loved { score *= 2.0; }
    
    score
}
```

### Weight Function

The weighting system gives new songs advantages:

- **< 5 touches**: Listens 4x more important than skips (encourages exploration)
- **5-15 touches**: Equal weighting (balanced learning)  
- **> 15 touches**: Skips 4x more important (stable preferences)

### Dampening Function

For songs with > 30 touches, logarithmic dampening prevents score inflation:

```rust
fn dampen(touches: u32) -> f64 {
    f64::from(touches + 1).log(1.2)
}
```

### Connection Weighting

Connections between songs are weighted using the same logarithmic approach:

```rust
fn apply_connection_weight(base_score: f64, connection_count: u32) -> f64 {
    if connection_count == 0 { return base_score; }
    let connection_weight = (connection_count as f64 + 1.0).log(1.2);
    base_score * connection_weight
}
```

## 🗄️ Database Design

### Schema Rationale

**Songs Table**:
- `id`: Primary key for efficient joins
- `path`: Unique file path (indexed)
- `artist`, `album`, `title`: Metadata for display and search
- `touches`, `listens`, `skips`: Learning algorithm data
- `loved`: User preference override

**Connections Table**:
- `from_song_id`, `to_song_id`: Song relationship (composite primary key)
- `count`: Strength of connection (how often this transition occurs)

### Performance Considerations

- **Indexes**: Automatic indexing on `songs.path` and `connections.from_song_id`
- **Queries**: Most queries use indexed columns for O(log n) performance
- **Storage**: Efficient SQLite storage with minimal overhead

### Future Extensions

Potential schema additions:
- `genres` table for explicit genre classification
- `playlists` table for user-created playlists
- `listening_sessions` table for time-based analysis
- `mood_tags` table for mood-based selection

## 🚀 Performance Optimizations

### Compile-Time Optimizations

- **LTO**: Link-time optimization reduces binary size and improves performance
- **Single Codegen Unit**: Better optimization at cost of compile time
- **Release Profile**: Maximum optimization level

### Runtime Optimizations

- **Lazy Loading**: Songs loaded only when needed
- **Efficient Queries**: SQL queries optimized for common operations
- **Minimal Dependencies**: Only 8 core dependencies reduce startup time
- **Connection Pooling**: Single database connection per operation

### Memory Usage

- **Small Footprint**: ~10MB typical memory usage
- **Streaming Processing**: Large libraries processed incrementally
- **Efficient Data Structures**: Rust's zero-cost abstractions

## 🧪 Testing Strategy

### Unit Tests (To Be Implemented)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_calculation() {
        let song = Song {
            touches: 10,
            listens: 8,
            skips: 2,
            loved: false,
            // ... other fields
        };
        
        let score = calculate_score(&song);
        assert!(score > 0.0);
    }

    #[test]
    fn test_weight_function() {
        let (weight_listens, weight_skips) = weight(3);
        assert_eq!(weight_listens, 4);
        assert_eq!(weight_skips, 1);
    }
}
```

### Integration Tests (To Be Implemented)

- Database operations
- Queue generation
- MPD integration
- File system scanning

### Manual Testing

```bash
# Test database operations
./target/release/muse update ./test_music
./target/release/muse list

# Test queue generation
./target/release/muse current "Test Song"

# Test MPD integration
mpc status
./target/release/muse play algorithm
```

## 🔄 Development Workflow

### Feature Development

1. **Create Branch**: `git checkout -b feature/new-feature`
2. **Implement**: Write code following existing patterns
3. **Test**: Manual testing with real music library
4. **Document**: Update relevant documentation
5. **Review**: Code review process
6. **Merge**: Merge to main branch

### Code Style

```rust
// Use descriptive variable names
let song_connections = get_song_connections(song_id)?;

// Prefer explicit error handling
let result = operation_that_might_fail()
    .context("Descriptive error message")?;

// Document complex algorithms
/// Calculates song score using the simple algorithm from the PDF.
/// Songs with < 30 touches use weighted scoring, others use dampening.
fn calculate_score(song: &Song) -> f64 {
    // Implementation...
}
```

### Error Handling

- Use `anyhow::Result<T>` for all fallible operations
- Provide context with `.context()` for user-friendly error messages
- Fail fast on configuration errors
- Graceful degradation for non-critical features

## 🔧 Build and Deployment

### Local Development

```bash
# Quick development cycle
cargo watch -x "check --workspace"

# Test with real data
RUST_LOG=info cargo run -- update ~/Music
RUST_LOG=debug cargo run -- current "Artist - Song"
```

### Release Build

```bash
# Full release build
cargo build --release

# Strip additional symbols (Linux)
strip target/release/muse

# Verify binary
./target/release/muse --help
```

### Distribution

Currently manual distribution. Future considerations:
- Cargo crates.io publication
- Package manager integration (Homebrew, AUR)
- Statically linked binaries for easy distribution

## 🔍 Debugging

### Logging

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/muse command

# Module-specific logging
RUST_LOG=muse::algorithm=trace ./target/release/muse play

# Log to file
RUST_LOG=info ./target/release/muse play 2> muse.log
```

### Common Issues

**Database Locked**:
- Check if another Muse instance is running
- Verify database file permissions

**MPD Connection Failed**:
- Ensure MPD is running: `systemctl status mpd`
- Test connection: `mpc status`
- Check MPD configuration

**No Songs Found**:
- Verify music directory path
- Check file extensions (currently supports FLAC, MP3, OGG, M4A, WAV)
- Review metadata extraction logic

## 📈 Future Development

### Short-term Goals

1. **Comprehensive Testing**: Unit and integration tests
2. **Improved Metadata**: Use proper audio metadata libraries
3. **Better MPD Integration**: Direct protocol instead of mpc
4. **Performance Monitoring**: Benchmarks and profiling

### Medium-term Goals

1. **Plugin System**: Extensible algorithm framework
2. **Advanced Metadata**: Genre, mood, energy level analysis
3. **Time-based Learning**: Time-of-day and seasonal preferences
4. **Import/Export**: Backup and restore learning data

### Long-term Goals

1. **Machine Learning**: Advanced recommendation algorithms
2. **Cross-platform**: Windows and macOS optimization
3. **Network Features**: Sync learning data across devices
4. **Advanced Analytics**: Detailed listening pattern analysis

## 🤝 Contributing Guidelines

### Code Contributions

1. **Fork Repository**: Create personal fork for development
2. **Follow Style**: Use `cargo fmt` and `cargo clippy`
3. **Write Tests**: Add tests for new functionality
4. **Update Documentation**: Keep docs in sync with code changes
5. **Small PRs**: Prefer small, focused pull requests

### Documentation Contributions

- Fix typos and improve clarity
- Add examples for complex features
- Update architecture docs for code changes
- Translate documentation to other languages

### Bug Reports

Include in bug reports:
- Rust version (`rustc --version`)
- Operating system and version
- MPD version (`mpd --version`)
- Steps to reproduce
- Expected vs actual behavior
- Relevant log output

---

**Happy coding! 🎵**