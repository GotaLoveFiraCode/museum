# Muse v2 Algorithm Documentation

This document provides detailed explanations of the algorithms used in Muse v2, based on the original design specification from Muse-V2.pdf.

## 🧠 Algorithm Overview

Muse uses a dual-algorithm approach to provide intelligent music suggestions:

1. **Simple Algorithm**: Scores individual songs based on listening history
2. **Complex Algorithm**: Tracks song-to-song relationships and connections

Both algorithms work together to create personalized playlists that balance exploration of new music with exploitation of known preferences.

## 📊 Simple Algorithm

The Simple Algorithm forms the foundation of Muse's scoring system. It calculates a score for each song based on the user's interaction history.

### Core Metrics

Three key metrics track user interaction with each song:

- **Touches**: How many times the algorithm has suggested this song
- **Listens**: How many times the user listened to the entire song
- **Skips**: How many times the user skipped this song

### Algorithm Logic

```rust
fn calculate_score(song: &Song) -> f64 {
    let mut score = if song.touches < 30 {
        // New song: Use dynamic weighting
        let (weight_listens, weight_skips) = weight(song.touches);
        (weight_listens as f64 * song.listens as f64) - 
        (weight_skips as f64 * song.skips as f64)
    } else {
        // Established song: Use logarithmic dampening
        let dampening = dampen(song.touches);
        dampening * song.listens as f64 - dampening * song.skips as f64
    };
    
    // Clamp negative scores to zero
    if score < 0.0 {
        score = 0.0;
    }
    
    // Boost loved songs
    if song.loved {
        score *= 2.0;
    }
    
    score
}
```

### Dynamic Weighting System

The weighting function provides different treatment based on song experience:

```rust
fn weight(touches: u32) -> (u8, u8) {
    let (low, medium, high) = (1, 2, 4);
    let (small_threshold, big_threshold) = (5, 15);
    
    if touches < small_threshold {
        // Early exploration: Favor listens heavily
        (high, low)  // (4, 1)
    } else if touches <= big_threshold {
        // Learning phase: Balanced approach
        (medium, medium)  // (2, 2)
    } else {
        // Stable preferences: Skips become more important
        (low, high)  // (1, 4)
    }
}
```

#### Weighting Phases Explained

**Phase 1: Early Exploration (< 5 touches)**
- Listen weight: 4, Skip weight: 1
- **Rationale**: Early skips might be anecdotal (wrong mood, interruption)
- **Effect**: New songs get multiple chances to prove themselves

**Phase 2: Learning (5-15 touches)**
- Listen weight: 2, Skip weight: 2
- **Rationale**: Balanced evaluation as patterns emerge
- **Effect**: Equal consideration of positive and negative feedback

**Phase 3: Stable Preferences (> 15 touches)**
- Listen weight: 1, Skip weight: 4
- **Rationale**: Patterns are established, skips indicate real dislike
- **Effect**: Algorithm learns to avoid songs you consistently skip

### Logarithmic Dampening

For songs with ≥ 30 touches, the algorithm switches to logarithmic dampening:

```rust
fn dampen(touches: u32) -> f64 {
    // Base 1.2 logarithm prevents score inflation
    f64::from(touches + 1).log(1.2)
}
```

**Purpose**: Prevents older songs from accumulating infinitely high scores

**Mathematical Effect**:
- 30 touches → dampening factor ≈ 7.8
- 100 touches → dampening factor ≈ 12.3
- 1000 touches → dampening factor ≈ 21.5

**Why Base 1.2?**: Provides gentle dampening that still rewards frequent listening without causing runaway scores.

### Examples

**New Song (3 touches, 2 listens, 1 skip)**:
```
weight = (4, 1)  # Early exploration phase
score = (4 × 2) - (1 × 1) = 8 - 1 = 7
```

**Learning Song (10 touches, 6 listens, 4 skips)**:
```
weight = (2, 2)  # Learning phase
score = (2 × 6) - (2 × 4) = 12 - 8 = 4
```

**Established Song (50 touches, 35 listens, 15 skips)**:
```
dampening = log₁.₂(51) ≈ 9.2
score = 9.2 × 35 - 9.2 × 15 = 322 - 138 = 184
```

**Loved Song (any stats)**:
```
base_score = calculate_as_above()
final_score = base_score × 2.0
```

## 🔗 Complex Algorithm

The Complex Algorithm tracks relationships between songs to understand musical flow and user preferences for song sequences.

### Connection Tracking

The system maintains a connections table that records song transitions:

```sql
CREATE TABLE connections (
    from_song_id INTEGER,
    to_song_id INTEGER,
    count INTEGER DEFAULT 1,
    PRIMARY KEY (from_song_id, to_song_id)
);
```

**How Connections Form**:
1. User listens to Song A completely
2. Song B starts playing next
3. Connection A → B is recorded or strengthened

### Connection Scoring

Connection strength uses the same logarithmic approach as the Simple Algorithm:

```rust
fn apply_connection_weight(base_score: f64, connection_count: u32) -> f64 {
    if connection_count == 0 {
        return base_score;
    }
    
    let connection_weight = (connection_count as f64 + 1.0).log(1.2);
    base_score * connection_weight
}
```

**Example**: If Song A → Song B has been observed 10 times:
```
connection_weight = log₁.₂(11) ≈ 3.2
final_score = base_score × 3.2
```

### Usage in Queue Generation

The Complex Algorithm is used differently in each queue type:

**Current Queues**: Use top 2 connections to create dual paths
**Thread Queues**: Follow single strongest connection chain  
**Stream Queues**: Randomly select from positive-score connections

## 🔀 Queue Generation Algorithms

### Current Queue Algorithm

Creates varied queues by mixing two connection paths:

```rust
fn generate_current(starting_song: &Song) -> Vec<QueuedSong> {
    // 1. Get all connections from starting song
    let connections = get_song_connections(starting_song.id);
    
    // 2. Score and sort connections
    let scored_connections: Vec<_> = connections
        .into_iter()
        .map(|(song, count)| {
            let base_score = calculate_score(&song);
            let final_score = apply_connection_weight(base_score, count);
            (song, final_score)
        })
        .sorted_by_score()
        .collect();
    
    // 3. Take top 2 connections
    let top_two = scored_connections.take(2);
    
    // 4. Generate paths from each
    let path1 = generate_path(top_two[0].id, 4);
    let path2 = generate_path(top_two[1].id, 4);
    
    // 5. Interleave paths
    let mut queue = vec![starting_song];
    for i in 0..max(path1.len(), path2.len()) {
        if i < path1.len() { queue.push(path1[i]); }
        if i < path2.len() { queue.push(path2[i]); }
    }
    
    queue
}
```

**Result**: Queue alternates between two musical directions from the starting song.

### Thread Queue Algorithm

Follows single strongest connection path:

```rust
fn generate_thread(starting_song: &Song) -> Vec<QueuedSong> {
    let mut queue = vec![starting_song];
    let path = generate_path(starting_song.id, 8);
    queue.extend(path);
    
    // Ensure minimum length with random high-scoring songs
    while queue.len() < 9 {
        if let Some(random_song) = get_random_high_scoring_song() {
            queue.push(random_song);
        }
    }
    
    queue.truncate(27);  // Maximum length
    queue
}
```

**Result**: Coherent musical journey following strongest relationships.

### Stream Queue Algorithm

Training-focused with controlled randomness:

```rust
fn generate_stream(starting_song: &Song) -> Vec<QueuedSong> {
    let mut queue = vec![starting_song];
    let mut current_id = starting_song.id;
    
    while queue.len() < 30 {
        let connections = get_song_connections(current_id);
        
        let next_song = if connections.len() < 3 {
            // Insufficient connections: add random song
            get_random_high_scoring_song()
        } else {
            // Randomly select from positive-score connections
            let positive_connections: Vec<_> = connections
                .into_iter()
                .filter(|(song, _)| calculate_score(song) > 0.0)
                .collect();
            
            if positive_connections.is_empty() {
                get_random_high_scoring_song()
            } else {
                randomly_select_from(positive_connections)
            }
        };
        
        if let Some(song) = next_song {
            current_id = song.id;
            queue.push(song);
        }
    }
    
    queue
}
```

**Result**: 30-song queue balancing learned preferences with exploration.

### Path Generation Helper

Core function for following connection chains:

```rust
fn generate_path(start_id: i64, max_length: usize) -> Vec<QueuedSong> {
    let mut path = Vec::new();
    let mut current_id = start_id;
    
    for _ in 0..max_length {
        let connections = get_song_connections(current_id);
        if connections.is_empty() {
            break;
        }
        
        // Find highest-scoring connection
        let best_song = connections
            .into_iter()
            .map(|(song, count)| {
                let base_score = calculate_score(&song);
                let final_score = apply_connection_weight(base_score, count);
                (song, final_score)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(song, _)| song);
        
        if let Some(song) = best_song {
            if calculate_score(&song) <= 0.0 {
                break;  // Stop at negative scores
            }
            
            current_id = song.id;
            path.push(QueuedSong::from(song));
        } else {
            break;
        }
    }
    
    path
}
```

## 🎯 Algorithm Tuning Parameters

### Critical Constants

**Touch Threshold (30)**:
- Switches from weighted to dampened scoring
- **Rationale**: 30 suggestions provides enough data for stable patterns
- **Effect**: Prevents new song bias from lasting too long

**Weight Values (1, 2, 4)**:
- Provides 2x and 4x multipliers for different phases
- **Rationale**: Significant but not extreme preference differences
- **Effect**: Clear phase transitions without sudden jumps

**Touch Thresholds (5, 15)**:
- Define transitions between weighting phases
- **Rationale**: 5 touches = initial impression, 15 touches = established pattern
- **Effect**: Smooth progression from exploration to exploitation

**Logarithm Base (1.2)**:
- Controls dampening aggressiveness
- **Rationale**: Gentle curve that doesn't overly penalize frequent songs
- **Effect**: Maintains variety without completely ignoring popular songs

**Queue Lengths (9-27, 30)**:
- Current/Thread: Variable length based on available connections
- Stream: Fixed 30 for consistent training
- **Rationale**: Enough variety without overwhelming choice

### Performance Implications

**Database Queries**:
- Simple Algorithm: O(1) per song
- Complex Algorithm: O(log n) per connection lookup
- Queue Generation: O(k log n) where k = queue length

**Memory Usage**:
- Scores calculated on demand (no caching)
- Connections loaded as needed
- Queue generation working set scales with connection density

**Optimization Opportunities**:
- Score caching for stable songs
- Connection strength precomputation  
- Batch queue generation

## 🔍 Algorithm Behavior Analysis

### Cold Start Problem

**Challenge**: New users have no listening history

**Solution**:
1. All songs start with score 0
2. Weight function heavily favors listens over skips initially
3. Stream queues provide exploration opportunities
4. Random song selection ensures discovery

### Exploration vs Exploitation Balance

**Exploration Mechanisms**:
- New song advantage in weighting
- Random song injection in Stream queues
- Multiple paths in Current queues

**Exploitation Mechanisms**:
- Connection strength bonuses
- Logarithmic reward for frequent listening
- Loved song multipliers

### Feedback Loop Stability

**Positive Feedback**:
- Good songs → more plays → higher scores → more suggestions
- **Risk**: Filter bubble effect

**Negative Feedback**:
- Skip penalty increases with experience
- Dampening prevents runaway scores
- Random injection provides escape routes

**Balance Mechanisms**:
- Phase-based weighting prevents early lock-in
- Connection diversity in Current queues
- Training focus of Stream queues

## 📈 Algorithm Evolution

### Learning Curve

**Phase 1 (0-100 songs)**: Heavy exploration
- Most songs have < 5 touches
- High variety, some misses
- Rapid learning from user feedback

**Phase 2 (100-1000 songs)**: Pattern formation
- Connection graph develops
- Stable preferences emerge
- Queue quality improves

**Phase 3 (1000+ songs)**: Mature recommendations
- Rich connection network
- Predictable but varied suggestions
- Fine-tuned to user preferences

### Long-term Behavior

**Advantages**:
- Continuously adapts to changing preferences
- Maintains variety through connection diversity
- Handles large libraries effectively

**Considerations**:
- May become conservative with age
- Requires periodic Stream queue use for freshness
- Connection graph can become sparse for rarely-played songs

## 🛠️ Implementation Notes

### Numerical Stability

**Score Clamping**: Negative scores set to 0.0 prevents undefined behavior

**Integer Overflow**: Touch/listen/skip counters use u32 (4+ billion capacity)

**Floating Point**: f64 provides sufficient precision for score calculations

### Edge Cases

**No Connections**: Fall back to random high-scoring songs

**All Negative Scores**: Queue generation includes random elements

**Database Empty**: Graceful error handling with helpful messages

**File Missing**: Continue queue generation, log warnings

### Future Enhancements

**Temporal Factors**:
- Time-of-day preferences
- Seasonal listening patterns
- Recently played avoidance

**Advanced Scoring**:
- Multiple genre support
- Mood classification
- Energy level matching

**Machine Learning**:
- Neural network scoring
- Collaborative filtering
- Advanced feature extraction

---

**The algorithms in Muse v2 represent a careful balance of simplicity and sophistication, providing personalized music recommendations that improve with use while maintaining musical discovery and variety.**