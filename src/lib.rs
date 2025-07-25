//! Intelligent music player that learns from listening habits.
//!
//! Core modules:
//! - [`algorithm`] - Song scoring algorithms
//! - [`db`] - Database operations  
//! - [`queue`] - Queue generation (current, thread, stream)
//! - [`daemon`] - Real-time behavior tracking
//! - [`mpd_client`] - MPD integration
//! 
//! ### Supporting Modules
//! 
//! - [`config`] - Configuration and data directory management
//! - [`cli`] - Command-line interface definitions with clap integration
//! - [`completion`] - Shell completion generation for enhanced UX
//! - [`path_translator`] - Bidirectional path conversion utilities
//! - [`mpd_config`] - MPD configuration parsing and management
//! 
//! ## Quick Start Example
//! 
//! ```no_run
//! use muse::{db, algorithm, queue, daemon};
//! use anyhow::Result;
//! 
//! // Initialize and populate database
//! let db_path = muse::config::get_db_path()?;
//! db::init_database(&db_path, false, true)?;
//! 
//! // Calculate score for a song
//! let song = db::Song {
//!     id: 1,
//!     path: "/music/artist/album/song.flac".to_string(),
//!     artist: "Test Artist".to_string(),
//!     album: "Test Album".to_string(),
//!     title: "Test Song".to_string(),
//!     touches: 10,
//!     listens: 8,
//!     skips: 2,
//!     loved: true,
//! };
//! 
//! let context = algorithm::ScoringContext::default();
//! let score = algorithm::calculate_score_functional(&song, &context);
//! println!("Song score: {:.3}", score);
//! 
//! // Generate a queue
//! let generator = queue::QueueGeneratorV2::new(&db_path.to_string_lossy())?;
//! let current_queue = generator.generate_current("Test Song")?;
//! println!("Generated queue with {} songs", current_queue.len());
//! 
//! // Start behavior tracking daemon
//! let mut daemon = daemon::BehaviorDaemon::new()?;
//! // daemon.start_monitoring()?; // Runs indefinitely
//! 
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ## Algorithm Details
//! 
//! Muse uses a dual-algorithm approach combining simple statistical analysis with
//! complex connection-based recommendations:
//! 
//! ### Simple Algorithm
//! - Scores songs based on listen/skip ratios
//! - Applies early exploration bonuses for new songs (touches < 30)
//! - Uses logarithmic dampening for established songs (touches ≥ 30)
//! - Multiplies loved songs by configurable factor (default 2.0x)
//! 
//! ### Complex Algorithm
//! - Tracks song-to-song connection strengths
//! - Builds weighted connection graphs from listening patterns
//! - Enhances base scores using connection weights
//! - Enables coherent musical journey generation
//! 
//! ## Queue Generation Strategies
//! 
//! ### Current Queues (Dual-Path)
//! ```no_run
//! # use muse::queue::QueueGeneratorV2;
//! # let generator = QueueGeneratorV2::new("test.db").unwrap();
//! // Generates 9-27 songs following two strongest connections
//! let queue = generator.generate_current("Starting Song")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ### Thread Queues (Single-Path)
//! ```no_run
//! # use muse::queue::QueueGeneratorV2;
//! # let generator = QueueGeneratorV2::new("test.db").unwrap();
//! // Generates coherent single-path journey
//! let queue = generator.generate_thread("Starting Song")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ### Stream Queues (Training)
//! ```no_run
//! # use muse::queue::QueueGeneratorV2;
//! # let generator = QueueGeneratorV2::new("test.db").unwrap();
//! // Generates exactly 30 songs with controlled randomness
//! let queue = generator.generate_stream("Starting Song")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ## Behavior Tracking
//! 
//! The daemon automatically tracks user behavior:
//! 
//! - **Touches**: Incremented when a song starts playing (after 3 seconds)
//! - **Listens**: Incremented when a song plays >80% before changing
//! - **Skips**: Incremented when a song plays ≤80% before changing
//! - **Connections**: Updated when songs transition naturally (not on skip)
//! - **Love Status**: Manual user marking for explicit preferences
//! 
//! ## Error Handling
//! 
//! All public functions return `Result<T, anyhow::Error>` for comprehensive error
//! handling. Common error scenarios include:
//! 
//! - Database connection failures
//! - MPD communication errors
//! - File system permission issues
//! - Song not found in database
//! - Invalid configuration values
//! 
//! ## Performance Characteristics
//! 
//! - **Database Operations**: Optimized with proper indexing, typically <1ms
//! - **Algorithm Scoring**: Functional implementation, ~100μs per song
//! - **Queue Generation**: 9-30 songs generated in <10ms
//! - **Memory Usage**: ~10MB typical, scales with database size
//! - **Daemon Overhead**: Minimal CPU usage, event-driven architecture
//! 
//! ## Testing
//! 
//! The library includes comprehensive testing:
//! - Unit tests for all modules (100% coverage goal)
//! - Integration tests for CLI workflows
//! - Performance benchmarks for critical paths
//! - Property-based testing for algorithm invariants
//! 
//! Run tests with:
//! ```bash
//! cargo test
//! cargo test --release  # For performance tests
//! ```

pub mod algorithm;
pub mod cli;
pub mod completion;
pub mod config;
pub mod daemon;
pub mod db;
pub mod mpd_client;
pub mod mpd_config;
pub mod path_translator;
pub mod queue;