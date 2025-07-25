# Muse: Unleashing Music v2

> **Offline Music Suggestions & Playlists**

Muse is an intelligent music player that learns from your listening habits to provide personalized music suggestions without requiring internet connectivity or streaming services. Version 2 is a complete rewrite focused on CLI interface and MPD integration.

## 🎵 What is Muse?

Muse solves the problem of finding good background music from your local music library. Instead of random shuffling, it uses sophisticated algorithms to:

- **Learn your preferences** by tracking what you listen to and skip
- **Create intelligent playlists** based on song-to-song connections
- **Provide three queue types** with different levels of exploration vs. exploitation
- **Work completely offline** with your existing music collection

## ✨ Key Features

### 🧠 Intelligent Algorithms
- **Simple Algorithm**: Scores songs based on listening history with dynamic weighting
- **Complex Algorithm**: Tracks song-to-song connections to understand your flow preferences
- **Adaptive Learning**: Gives new songs advantages while learning from established patterns

### 📋 Three Queue Types
- **Current**: Dual-path queue mixing two connection chains (9-27 songs)
- **Thread**: Single-path queue following one connection chain (9-27 songs)  
- **Stream**: Long queue with randomness for algorithm training (exactly 30 songs)

### 🎛️ MPD Integration
- Uses Music Player Daemon for robust audio playback
- Compatible with existing MPD setups and clients
- Manages queues through standard MPD protocol

### 📊 Learning System
- Tracks "touches" (times suggested), "listens" (completed plays), and "skips"
- Builds connection graphs between songs
- Supports "loved" songs for explicit preference marking

## 🚀 Quick Start

### Prerequisites
- **Rust 1.70+** for building
- **MPD (Music Player Daemon)** running on localhost:6600
- **mpc** command-line tool for MPD communication

### Installation

```bash
# Clone repository
git clone <repository-url>
cd museum
git checkout v2-rewrite

# Build release version
cargo build --release

# Binary will be at target/release/muse
```

### First Run

```bash
# 1. Update your music database
./target/release/muse update /path/to/your/music/directory

# 2. List your songs to verify
./target/release/muse list

# 3. Start the behavior tracking daemon (IMPORTANT!)
./target/release/muse daemon start

# 4. Start playing with the algorithm
./target/release/muse play algorithm

# 5. Generate queues with verbose output to see algorithm decisions
./target/release/muse stream "Song Name" -v
```

### New in v2.1: Real-Time Behavior Tracking

The daemon now provides **automatic behavior tracking** without requiring special commands:

```bash
# Start daemon once (runs in background)
muse daemon start

# Generate any queue - tracking happens automatically!
muse current "Artist - Song"

# See real-time notifications:
# ♫ PLAYING: Artist - Song (touch #5)
# ✓ LISTENED: Artist - Song (listens: 3, skips: 1)
# ✗ SKIPPED: Artist - Song (listens: 3, skips: 2)
```

## 📖 Usage Guide

### Commands Overview

```bash
muse <COMMAND>

Commands:
  update   Update the music database from a directory
  list     List all songs in the database
  play     Play music using MPD [shuffle|algorithm]
  current  Generate and play a "current" queue based on a song
  thread   Generate and play a "thread" queue based on a song
  stream   Generate and play a "stream" queue based on a song
  next     Skip to next track with behavior tracking
  skip     Skip current track with explicit skip tracking
  love     Mark current track as loved
  unlove   Remove loved status from current track
  info     Show detailed information about the current song
  daemon   Manage the behavior tracking daemon [start|stop|status]
  help     Print this message or the help of the given subcommand(s)
```

### New Features in v2.1

#### 🔄 **Automatic Behavior Tracking**
The daemon monitors MPD events in real-time:
- Tracks when songs start (touches)
- Detects natural listens (>80% played) vs skips (≤80% played)
- Updates connection weights automatically
- Works with any MPD client (ncmpcpp, mpc, etc.)

#### 📊 **Verbose Queue Generation**
Add `-v` flag to see algorithm decisions:
```bash
muse stream "Song Name" -v
# Output:
# 🎵 Generating Stream Queue (training mode)
# 📊 Starting song search: Song Name
# ✅ Generated 30 songs
#   1. Artist - Title (score: 2.145, touches: 12, L/S: 8/3) ❤️
#   2. Artist2 - Title2 (score: 1.892, touches: 5, L/S: 4/1)
```

#### 📀 **Current Song Information**
Get detailed stats about what's playing:
```bash
muse info
# Output:
# 📀 Currently Playing
# ♫ Song: Title
# 👤 Artist: Artist  
# ⏱️ Time: 02:15/03:45
# 📊 Algorithm Statistics
# 🎯 Current Score: 2.145
# 👆 Touches: 12
# ✅ Listens: 8
# ⏭️ Skips: 3
# 📈 Listen Rate: 67%
# 🔗 Top Connections (shows related songs)
```

### Database Management

```bash
# Update database with new music
muse update /path/to/music

# View all catalogued songs
muse list
```

### Playback Modes

```bash
# Play with intelligent algorithm (recommended)
muse play algorithm

# Play with simple shuffle
muse play shuffle

# Use default (algorithm)
muse play
```

### Queue Generation

Generate specific queue types based on a starting song:

```bash
# Current queue (dual-path, varied)
muse current "Song Title"
muse current "Artist Name"

# Thread queue (single-path, focused)
muse thread "Song Title"

# Stream queue (long, training-focused)
muse stream "Song Title"
```

## 🔧 Configuration

### Data Storage
- Database: `~/.local/share/muse/music.db` (Linux) or equivalent system data directory
- No configuration files needed - works out of the box

### MPD Setup
Ensure MPD is configured and running:

```bash
# Check MPD status
systemctl status mpd

# Test MPD connection
mpc status

# Start MPD if needed
systemctl start mpd
```

## 🎯 How It Works

### The Learning Process

1. **Initial Phase**: New songs get preference to build listening data
2. **Connection Building**: System tracks which songs play well together
3. **Preference Learning**: Balances your skip/listen patterns with connections
4. **Intelligent Suggestions**: Uses both algorithms to suggest next songs

### Scoring Algorithm

Songs are scored using a multi-factor system:

- **Touches < 30**: Uses weighted system favoring new content
- **Touches ≥ 30**: Uses logarithmic dampening for stability
- **Connections**: Multiplies base score by connection strength
- **Loved Songs**: Get 2x score boost

### Queue Types Explained

**Current Queues**: Best for daily listening
- Takes top 2 connections from starting song
- Builds two separate paths
- Interleaves them for variety
- 9-27 songs depending on available connections

**Thread Queues**: Best for focused listening
- Follows single strongest connection path
- More coherent mood/genre
- Same length as Current queues

**Stream Queues**: Best for training the algorithm
- Always exactly 30 songs
- Introduces more randomness
- Helps build new connections
- Recommended for new users

## 🎵 Best Practices

### For New Users
1. Start with `stream` queues to train the algorithm
2. Use MPD clients like `ncmpcpp` for playback control
3. Let songs play completely when you enjoy them
4. Skip songs you don't want to hear again

### For Established Libraries
1. Use `current` queues for daily background music
2. Use `thread` queues when you want consistent mood
3. Periodically use `stream` queues to discover forgotten music
4. Mark favorite songs as "loved" in the database

### Algorithm Training
- **Skip early** if you don't like a song (helps algorithm learn)
- **Play completely** songs you enjoy (builds positive connections)
- **Use variety** in your queue generation starting points
- **Be patient** - the algorithm improves with more data

## 🔍 Examples

### Typical Workflow

```bash
# Morning: Start with a mood-setting song
muse current "Debussy - Clair de Lune"

# Afternoon: Continue with more energetic music
muse thread "The Beatles - Come Together"

# Evening: Let the algorithm surprise you
muse stream "Miles Davis - Kind of Blue"

# Check what's in your database
muse list | grep "Beatles"
```

### Library Management

```bash
# Add new album
muse update /home/user/Music/NewAlbum

# Update entire library
muse update /home/user/Music

# See recent additions
muse list | tail -20
```

## 🆚 Version Differences

### V2 vs V1 (Alpha)
- ✅ **Removed GUI**: CLI-only for better performance
- ✅ **MPD Integration**: Professional audio daemon instead of direct playback
- ✅ **Minimal Dependencies**: 8 crates vs 20+ in V1
- ✅ **Exact Algorithm**: Implements precise algorithm from design document
- ✅ **Better Architecture**: Modular design with clear separation
- ✅ **Optimized Builds**: LTO and aggressive optimization

## 📊 Performance

### System Requirements
- **RAM**: ~10MB typical usage
- **Storage**: ~1MB per 1000 songs in database
- **CPU**: Minimal when not generating queues

### Performance Benchmarks

Muse includes comprehensive performance benchmarks showing excellent efficiency:

```bash
# Run performance benchmarks
cargo bench

# Quick benchmark overview
./bench.sh
```

**Key Performance Metrics** (measured on modern hardware):
- **Song Scoring**: ~25ns per song (40M songs/second)
- **Batch Processing**: ~25μs for 1000 songs  
- **Database Queries**: <5ms for typical searches
- **Queue Generation**: <100ms for 30-song queues
- **Connection Weight**: ~5ns per calculation

### Optimization Features
- SQLite with proper indexing
- Lazy loading of song data
- Efficient connection queries
- Release builds with LTO
- Functional algorithms with zero-cost abstractions
- SIMD-optimized batch processing

## 🤝 Contributing

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development information.

### Quick Development Setup

```bash
git clone <repository>
cd museum
git checkout v2-rewrite
cargo check
cargo test  # (when tests are added)
```

## 🔧 Troubleshooting

### MPD Issues
```bash
# Check MPD status
systemctl status mpd    # System-wide
systemctl --user status mpd  # User-level

# Start MPD if not running
systemctl start mpd     # System-wide
systemctl --user start mpd   # User-level

# Check MPD is accessible
mpc status
```

### Database Issues
```bash
# Database location check
find ~/.local/share/ -name "music.db" 2>/dev/null

# Reinitialize if corrupted
muse update /path/to/music --remove-missing
```

### Path Translation Errors
- Ensure MPD and Muse see the same music files
- Check MPD config: `~/.config/mpd/mpd.conf`
- Verify music directory paths match

## 📚 Technical Documentation

- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Developer guide and architecture
- **[ALGORITHMS.md](ALGORITHMS.md)** - Detailed algorithm explanations
- **[API.md](API.md)** - Module and function documentation

## 📄 License

GPL-3.0-or-later

---

**Made for music lovers who want intelligent suggestions from their own collections** 🎶