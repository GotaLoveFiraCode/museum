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
- **mpc** command-line tool for MPD communication (Todo: MPD crate integration)

### Installation

```bash
# Clone repository
git clone <repository-url> # errr, umm, uhhh
cd museum

# Build release version
cargo build --release

# Benchmark for funsies
cargo bench

# Binary will be at target/release/muse
```

### First Run

You may want to set up a link to the binary like so:
```bash
ln -s <path>/museum/target/release/muse ~/.local/bin/muse
```
Obviously this needs `~/.local/bin` to be in your `PATH`.

```bash
# 1. Initialize your music database
./target/release/muse init-db /path/to/your/music/directory

# 2. List your songs to verify
# Song format is a little messed up right now, WIP
./target/release/muse list

# 3. Start the behavior tracking daemon (IMPORTANT!)
# Will automate this soon probably, WIP
./target/release/muse daemon start

# 4. Start playing with the algorithm
# I never use this, WIP
./target/release/muse play algorithm

# 5. Generate queues with verbose output to see algorithm decisions
# This is good in the beginning, I would start with this.
./target/release/muse stream "Song Name" -v

# 6. Once you have a informed database, try smarter queue-generators
./target/release/muse current "Song Name" -v
# or
./target/release/muse thread "Song Name" -v
```

You can skip using the `muse skip` command or `mpc next`.

Note that there is a bug in `mpc`, that causing `mpc next` to also pause the stream. Just run `mpc toggle && mpc toggle` to fix this.

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
  init-db              Initialize database from music directory (full scan). WARNING: resets all data!
  update               Update database with new files (incremental)
  list                 List all songs in the database. WIP.
  play                 Play music using MPD. Don’t use this, I don’t even remember implementing it...
  thread               Generate and play a "thread" queue based on a song (little variation)
  current              Generate and play a "current" queue based on a song (some variation)
  stream               Generate and play a "stream" queue based on a song (much variation)
  completion           Generate shell completions
  completion-enhanced  Generate enhanced completion with song name completion (only bash and fish, WIP)
  next                 Skip to next track with behavior tracking (not necessary when using daemon)
  skip                 Skip current track with explicit skip tracking (not necessary when using daemon)
  love                 Mark current track as loved (only use for _extreme_ boost to fix faulty behavior)
  unlove               Remove loved status from current track
  info                 Show detailed information about the current song (a little broken right now, WIP)
  daemon               Manage the behavior tracking daemon (necessary)
  help                 Print this message or the help of the given subcommand(s)
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

I hate emojis.

### Database Management

```bash
# Initialize database (first time)
muse init-db /path/to/music

# Update database with new music (incremental)
muse update /path/to/music

# View all catalogued songs (WIP)
muse list
```

### Playback Modes

```bash
# Play with intelligent algorithm (not recommended)
muse play algorithm

# Play with simple shuffle (also not recommended)
muse play shuffle

# Use default (algorithm, also not recommended, duh)
muse play
```

Please use queue generation when possible.

### Queue Generation

Generate specific queue types based on a starting song:

```bash
# Current queue (dual-path, varied)
muse current "Song Title"
muse current "Artist Name"

# Thread queue (single-path, focused)
muse thread "Song Title"

# Stream queue (long, training-focused, recommended for new users)
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
# or equivalent for runit, s8, etc. — e.g.
sv status mpd
# you may also run mpd as a command ofc; it doesn’t need to be a service.

# Test MPD connection
mpc status

# Start MPD if needed
systemctl start mpd
# or, for runit
sv up mpd
# etc.
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

This may be subject to change... and I’m always open to suggestions!

### Queue Types Explained (IMPORTANT)

**Current Queues**: Best for daily listening once algo/db has enough data
- Takes top 2 connections from starting song
- Builds two separate paths
- Interleaves them for variety
- 9-27 songs depending on available connections

**Thread Queues**: Best for focused listening once algo/db has a lot of data
- Follows single strongest connection path
- More coherent mood/genre
- Same length as Current queues

**Stream Queues**: Best for training the algorithm, i.e., best for newbies
- Always exactly 30 songs
- Introduces more randomness
- Helps build new connections
- Recommended for new users

## 🎵 Best Practices (also important)

### For New Users
1. Start with `stream` queues to train the algorithm
2. Use MPD clients like `ncmpcpp` or `mpc` for playback control
3. Let songs play completely when you enjoy them
4. Skip songs you don't want to hear again

### For Established Libraries
1. Use `current` queues for daily background music
2. Use `thread` queues when you want consistent mood
3. Periodically use `stream` queues to discover forgotten music
4. Mark songs that consistently don’t get suggested often enough or have been forgotten as “loved”

### Algorithm Training
- **Skip early** if you don't like a song or it doesn’t fit the mood (helps algorithm learn)
- **Play completely** songs you enjoy or fit the mood very well (builds positive connections)
- **Use variety** in your queue generation starting points
- **Be patient**; the algorithm improves with more data

## 🔍 Examples

### Typical Workflow

```bash
# Morning: Start with a mood-setting song
muse current "HEALTH - Zoothorns"
muse current "Bong-Ra - Dystopic"

# Afternoon: Continue with more energetic music
muse thread "~/Music/experimental/weir_bips_and_bops_with_BASS.flac" # path
muse thread "Q-Tip - Feelin'" # track
muse thread "J Dilla - Donuts" # album
muse thread "Run The Jewels" # artist

# Evening: Let the algorithm surprise you
muse stream "Muddy Waters - Long Distance Call"
muse stream "Verdi La Traviata" # fuzzy matching works as well

# Check what’s in your database
muse list | grep "John Chowning" | wc -l # how much music by chowning
muse stream $(muse list | fzf -q "Zeal & Ardor") # interactive selection
muse list | rg -P "^(?!.*Ambient).*Aphex Twin.*$" # Aphex but not the ambient works
```

### Library Management

```bash
# Add new album (incremental update)
muse update /home/user/Music/NewAlbum

# Update entire library (incremental)
muse update /home/user/Music

# Full re-scan of library (if needed); WARNING: resets all data in DB
muse init-db /home/user/Music

# See recent additions (WIP)
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

### Performance Benchmarks (WIP)

Muse includes comprehensive performance benchmarks showing excellent efficiency:

```bash
# Run performance benchmarks
cargo bench

# Quick benchmark overview
./bench.sh
```

**Key Performance Metrics** (measured on 32GB LPDDR6 RAM, AMD Ryzen 7 6800U (16@4.77GHz), and Samsung SSD 980 PRO with f2fs, all on Void Linux @ 6.15.7\_1):
- **Song Scoring**: ~25ns per song (40M songs/second)
- **Batch Processing**: ~25μs for 1000 songs  
- **Database Queries**: <5ms for typical searches
- **Queue Generation**: <100ms for 30-song queues
- **Connection Weight**: ~5ns per calculation

So it’s pretty fast.

## 🤝 Contributing

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development information.

### Quick Development Setup

```bash
git clone <repository>
cd museum
cargo clippy
cargo test
```

## 🔧 Troubleshooting

### MPD Issues
```bash
# Check MPD status
systemctl status mpd    # System-wide
systemctl --user status mpd  # User-level
# or equivalent for runit, s8, etc.

# Start MPD if not running
systemctl start mpd     # System-wide
systemctl --user start mpd   # User-level
# or equivalent for runit, s8, etc.

# Check MPD is accessible
mpc status
```

### Database Issues
```bash
# Database location check
find ~/.local/share/ -name "music.db" 2>/dev/null

# Reinitialize if corrupted
muse init-db /path/to/music
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
