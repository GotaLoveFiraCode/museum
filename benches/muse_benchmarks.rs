//! # Muse Performance Benchmarks
//! 
//! Comprehensive benchmarks for measuring the performance of critical Muse components.
//! These benchmarks help ensure that the system maintains high performance as it evolves.
//! 
//! ## Benchmark Categories
//! 
//! - **Algorithm Performance**: Scoring and batch processing benchmarks
//! - **Database Operations**: Query and update performance
//! - **Queue Generation**: Performance of different queue strategies
//! - **Connection Analysis**: Benchmark connection graph operations
//! - **Path Translation**: File path conversion performance
//! 
//! ## Running Benchmarks
//! 
//! ```bash
//! # Run all benchmarks
//! cargo bench
//! 
//! # Run specific benchmark group
//! cargo bench algorithm
//! cargo bench database
//! cargo bench queue
//! 
//! # Generate HTML reports
//! cargo bench -- --output-format html
//! ```

use criterion::{criterion_group, criterion_main, Criterion, BatchSize, BenchmarkId};
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;
use muse::{algorithm, db, queue, path_translator};

/// Helper function to create a test database with realistic data
fn create_benchmark_database() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("benchmark_music.db");
    
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
    
    // Create schema
    conn.execute(
        "CREATE TABLE songs (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            title TEXT NOT NULL,
            touches INTEGER DEFAULT 0,
            listens INTEGER DEFAULT 0,
            skips INTEGER DEFAULT 0,
            loved INTEGER DEFAULT 0
        )",
        [],
    ).expect("Failed to create songs table");
    
    conn.execute(
        "CREATE TABLE connections (
            id INTEGER PRIMARY KEY,
            from_song_id INTEGER NOT NULL,
            to_song_id INTEGER NOT NULL,
            count INTEGER DEFAULT 1,
            FOREIGN KEY (from_song_id) REFERENCES songs(id),
            FOREIGN KEY (to_song_id) REFERENCES songs(id),
            UNIQUE(from_song_id, to_song_id)
        )",
        [],
    ).expect("Failed to create connections table");
    
    // Create indexes for performance
    conn.execute("CREATE INDEX idx_songs_artist ON songs(artist)", [])
        .expect("Failed to create artist index");
    conn.execute("CREATE INDEX idx_songs_title ON songs(title)", [])
        .expect("Failed to create title index");
    conn.execute("CREATE INDEX idx_connections_from ON connections(from_song_id)", [])
        .expect("Failed to create connections from index");
    
    // Insert realistic test data (1000 songs for meaningful benchmarks)
    let mut stmt = conn.prepare(
        "INSERT INTO songs (path, artist, album, title, touches, listens, skips, loved) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    ).expect("Failed to prepare insert statement");
    
    for i in 1..=1000 {
        let artist_idx = (i - 1) / 50 + 1; // 20 artists, 50 songs each
        let album_idx = (i - 1) / 10 + 1; // 100 albums, 10 songs each
        let touches = (i % 100) as u32;
        let listens = (touches as f64 * 0.7) as u32; // 70% listen rate
        let skips = touches.saturating_sub(listens);
        let loved = i % 20 == 0; // 5% loved songs
        
        stmt.execute([
            &format!("/music/Artist{artist_idx}/Album{album_idx}/Song{i:04}.flac"),
            &format!("Artist {artist_idx}"),
            &format!("Album {album_idx}"),
            &format!("Song {i:04}"),
            &touches.to_string(),
            &listens.to_string(),
            &skips.to_string(),
            &(loved as u8).to_string(),
        ]).expect("Failed to insert song");
    }
    
    // Insert realistic connections (create a connected graph)
    let mut conn_stmt = conn.prepare(
        "INSERT OR IGNORE INTO connections (from_song_id, to_song_id, count) VALUES (?1, ?2, ?3)"
    ).expect("Failed to prepare connection insert");
    
    for i in 1..=500 {
        let from_id = i;
        let to_id = if i < 1000 { i + 1 } else { 1 }; // Create circular connections
        let count = (i % 10) + 1; // Connection strength 1-10
        
        conn_stmt.execute([from_id, to_id, count as i64])
            .expect("Failed to insert connection");
            
        // Add some random cross-connections
        if i % 5 == 0 {
            let random_to = ((i * 7) % 1000) + 1;
            if random_to != to_id { // Avoid duplicate connections
                conn_stmt.execute([from_id, random_to, (i % 5) + 1])
                    .expect("Failed to insert random connection");
            }
        }
    }
    
    (temp_dir, db_path)
}

/// Helper function to create test songs for algorithm benchmarks
fn create_test_songs(count: usize) -> Vec<db::Song> {
    (1..=count).map(|i| {
        let touches = (i % 100) as u32;
        let listens = (touches as f64 * 0.7) as u32;
        let skips = touches.saturating_sub(listens);
        
        db::Song {
            id: i as i64,
            path: format!("/music/test/song{i:04}.flac"),
            artist: format!("Artist {}", (i - 1) / 20 + 1),
            album: format!("Album {}", (i - 1) / 10 + 1),
            title: format!("Song {i:04}"),
            touches,
            listens,
            skips,
            loved: i % 20 == 0,
        }
    }).collect()
}

/// Benchmark algorithm scoring performance
fn benchmark_algorithm_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("algorithm_scoring");
    
    // Single song scoring
    let song = create_test_songs(1)[0].clone();
    let context = algorithm::ScoringContext::default();
    
    group.bench_function("single_song_score", |b| {
        b.iter(|| {
            algorithm::calculate_score_functional(black_box(&song), black_box(&context))
        })
    });
    
    // Batch scoring with different sizes
    for size in [10, 50, 100, 500, 1000].iter() {
        let songs = create_test_songs(*size);
        
        group.bench_with_input(
            BenchmarkId::new("batch_scoring", size),
            &songs,
            |b, songs| {
                b.iter(|| {
                    algorithm::batch_calculate_scores(black_box(songs), black_box(&context))
                        .collect::<Vec<_>>()
                })
            }
        );
    }
    
    // Connection weight calculation
    group.bench_function("connection_weight", |b| {
        b.iter(|| {
            algorithm::apply_connection_weight_advanced(
                black_box(10.0), 
                black_box(5), 
                black_box(1.1)
            )
        })
    });
    
    // Ranking large song sets
    let large_songs = create_test_songs(1000);
    group.bench_function("rank_1000_songs", |b| {
        b.iter_batched(
            || large_songs.clone(),
            |songs| algorithm::rank_songs_functional(black_box(songs), black_box(&context)),
            BatchSize::SmallInput
        )
    });
    
    group.finish();
}

/// Benchmark database operations
fn benchmark_database_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("database_operations");
    
    // Database creation and connection
    group.bench_function("database_creation", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp directory");
                let db_path = temp_dir.path().join("bench.db");
                (temp_dir, db_path)
            },
            |(_temp_dir, db_path)| {
                let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
                // Simple table creation
                conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", []).expect("Failed to create table");
                black_box(conn)
            },
            BatchSize::SmallInput
        )
    });
    
    // Query performance with realistic database
    let (_temp_dir, db_path) = create_benchmark_database();
    
    group.bench_function("song_search_by_title", |b| {
        b.iter(|| {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            let mut stmt = conn.prepare("SELECT * FROM songs WHERE title LIKE ?1 LIMIT 10").unwrap();
            let rows: Vec<_> = stmt.query_map(["%Song%"], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(4)?,
                ))
            }).unwrap().collect();
            black_box(rows)
        })
    });
    
    group.bench_function("song_search_by_artist", |b| {
        b.iter(|| {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            let mut stmt = conn.prepare("SELECT * FROM songs WHERE artist LIKE ?1 LIMIT 10").unwrap();
            let rows: Vec<_> = stmt.query_map(["%Artist 5%"], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(4)?,
                ))
            }).unwrap().collect();
            black_box(rows)
        })
    });
    
    group.bench_function("connection_lookup", |b| {
        b.iter(|| {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            let mut stmt = conn.prepare(
                "SELECT to_song_id, count FROM connections WHERE from_song_id = ?1 ORDER BY count DESC LIMIT 5"
            ).unwrap();
            let rows: Vec<_> = stmt.query_map([50], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            }).unwrap().collect();
            black_box(rows)
        })
    });
    
    group.bench_function("stats_update", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp directory");
                let db_path = temp_dir.path().join("update_bench.db");
                let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
                
                conn.execute(
                    "CREATE TABLE songs (
                        id INTEGER PRIMARY KEY,
                        touches INTEGER DEFAULT 0,
                        listens INTEGER DEFAULT 0,
                        skips INTEGER DEFAULT 0
                    )", []
                ).expect("Failed to create table");
                
                conn.execute("INSERT INTO songs (id) VALUES (1)", []).expect("Failed to insert data");
                (temp_dir, db_path)
            },
            |(_temp_dir, db_path)| {
                let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");  
                conn.execute(
                    "UPDATE songs SET touches = touches + 1, listens = listens + 1 WHERE id = 1",
                    []
                ).expect("Failed to update stats")
            },
            BatchSize::SmallInput
        )
    });
    
    group.finish();
}

/// Benchmark queue generation performance
fn benchmark_queue_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_generation");
    
    let (_temp_dir, db_path) = create_benchmark_database();
    let db_path_str = db_path.to_string_lossy().to_string();
    
    // Queue generator creation
    group.bench_function("generator_creation", |b| {
        b.iter(|| {
            queue::QueueGeneratorV2::new(black_box(&db_path_str))
        })
    });
    
    // Different queue generation strategies
    let generator = queue::QueueGeneratorV2::new(&db_path_str).unwrap();
    
    group.bench_function("current_queue", |b| {
        b.iter(|| {
            generator.generate_current(black_box("Song 0050")).unwrap_or_default()
        })
    });
    
    group.bench_function("thread_queue", |b| {
        b.iter(|| {
            generator.generate_thread(black_box("Song 0050")).unwrap_or_default()
        })
    });
    
    group.bench_function("stream_queue", |b| {
        b.iter(|| {
            generator.generate_stream(black_box("Song 0050")).unwrap_or_default()
        })
    });
    
    // Verbose queue generation (with output)
    group.bench_function("verbose_current_queue", |b| {
        b.iter(|| {
            generator.generate_current_verbose(black_box("Song 0050")).unwrap_or_default()
        })
    });
    
    group.finish();
}

/// Benchmark path translation operations
fn benchmark_path_translation(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_translation");
    
    let test_paths = vec![
        "/home/user/Music/Artist/Album/Song.flac",
        "/media/music/collection/Various Artists/Compilation/Track.mp3",
        "/mnt/storage/audio/Classical/Beethoven/Symphony.wav",
        "/Users/john/iTunes/Pop/Artist/Single.m4a",
        "/opt/music/library/Electronic/Album/Beat.ogg",
    ];
    
    let mpd_paths = vec![
        "Artist/Album/Song.flac",
        "collection/Various Artists/Compilation/Track.mp3",
        "Classical/Beethoven/Symphony.wav",
        "Pop/Artist/Single.m4a",
        "Electronic/Album/Beat.ogg",
    ];
    
    // Benchmark absolute to MPD relative conversion
    for (i, path) in test_paths.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("abs_to_mpd", i),
            path,
            |b, path| {
                b.iter(|| {
                    // This may fail in benchmark environment, but we're measuring the call overhead
                    let _ = path_translator::absolute_to_mpd_relative(black_box(path));
                })
            }
        );
    }
    
    // Benchmark MPD relative to absolute conversion
    for (i, path) in mpd_paths.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("mpd_to_abs", i),
            path,
            |b, path| {
                b.iter(|| {
                    // This may fail in benchmark environment, but we're measuring the call overhead
                    let _ = path_translator::mpd_relative_to_absolute(black_box(path));
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark song search and matching
fn benchmark_song_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("song_search");
    
    let (_temp_dir, db_path) = create_benchmark_database();
    
    // Search performance with different query types
    let search_queries = vec![
        "Song 0050",              // Exact title match
        "Artist 5",               // Artist search  
        "Album 25",               // Album search
        "Artist 5 - Song 0234",   // Combined format
        "song artist",            // Fuzzy search
    ];
    
    for (i, query) in search_queries.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("search_query", i),
            query,
            |b, query| {
                b.iter(|| {
                    let conn = rusqlite::Connection::open(&db_path).unwrap();
                    
                    // Simulate the multi-strategy search from the actual code
                    let mut stmt = conn.prepare(
                        "SELECT * FROM songs WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1 LIMIT 1"
                    ).unwrap();
                    
                    let pattern = format!("%{}%", query);
                    let result = stmt.query_row([&pattern], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(4)?,
                        ))
                    });
                    
                    black_box(result)
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark memory and allocation performance
fn benchmark_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    // Large data structure creation and manipulation
    group.bench_function("large_song_vector", |b| {
        b.iter(|| {
            let songs = create_test_songs(black_box(10000));
            black_box(songs)
        })
    });
    
    // String operations common in music metadata
    group.bench_function("string_operations", |b| {
        b.iter(|| {
            let title = black_box("Test Song Title");
            let artist = black_box("Test Artist Name");
            let album = black_box("Test Album Name");
            
            let combined = format!("{} - {} ({})", artist, title, album);
            let normalized = combined.to_lowercase();
            let parts: Vec<String> = normalized.split(" - ").map(|s| s.to_string()).collect();
            
            black_box((combined, normalized, parts))
        })
    });
    
    // Vector operations for queue management
    group.bench_function("queue_operations", |b| {
        b.iter(|| {
            let mut queue: Vec<i64> = (1..=30).collect();
            
            // Simulate queue manipulations
            queue.sort_by(|a, b| b.cmp(a)); // Reverse sort by score
            queue.truncate(25);              // Limit queue size
            queue.push(999);                 // Add new song
            queue.rotate_left(1);            // Rotate queue
            
            black_box(queue)
        })
    });
    
    group.finish();
}

// Group all benchmarks
criterion_group!(
    benches,
    benchmark_algorithm_scoring,
    benchmark_database_operations,
    benchmark_queue_generation,
    benchmark_path_translation,
    benchmark_song_search,
    benchmark_memory_operations
);

criterion_main!(benches);