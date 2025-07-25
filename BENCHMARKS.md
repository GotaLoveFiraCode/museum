# Muse Performance Benchmarks

This document describes the comprehensive benchmark suite for measuring Muse's performance across all critical components.

## Overview

The benchmark suite measures performance across six key areas:

1. **Algorithm Scoring** - Core song scoring and ranking algorithms
2. **Database Operations** - SQLite query and update performance
3. **Queue Generation** - Different queue strategy performance
4. **Path Translation** - File path conversion efficiency
5. **Song Search** - Multi-strategy search performance
6. **Memory Operations** - Memory allocation and data structure performance

## Running Benchmarks

### Quick Start

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark category
cargo bench algorithm_scoring
cargo bench database_operations
cargo bench queue_generation

# Run with HTML report generation
./bench.sh
```

### Detailed Commands

```bash
# Run benchmarks with specific parameters
cargo bench -- --sample-size 100
cargo bench -- --measurement-time 10

# Generate baseline for comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main

# Run only fast benchmarks
cargo bench -- --quick
```

## Benchmark Categories

### 1. Algorithm Scoring Benchmarks

**Purpose**: Measure the performance of core scoring algorithms that determine song recommendations.

**Benchmarks**:
- `single_song_score` - Individual song scoring performance (target: <100μs)
- `batch_scoring` - Batch processing of 10/50/100/500/1000 songs
- `connection_weight` - Connection weight calculation (target: <10μs)
- `rank_1000_songs` - Ranking large song collections (target: <50ms)

**Key Metrics**:
- Throughput: Songs scored per second
- Latency: Time per individual score calculation
- Scalability: Performance degradation with dataset size

### 2. Database Operations Benchmarks

**Purpose**: Measure SQLite database performance for critical operations.

**Benchmarks**:
- `database_creation` - Database and schema creation time
- `song_search_by_title` - Title-based song search queries
- `song_search_by_artist` - Artist-based song search queries
- `connection_lookup` - Song connection graph queries
- `stats_update` - Song statistics update operations

**Key Metrics**:
- Query latency (target: <5ms for searches)
- Update throughput (target: >1000 updates/second)
- Index effectiveness
- Connection query performance

### 3. Queue Generation Benchmarks

**Purpose**: Measure the performance of different queue generation strategies.

**Benchmarks**:
- `generator_creation` - Queue generator instantiation
- `current_queue` - Current (dual-path) queue generation
- `thread_queue` - Thread (single-path) queue generation
- `stream_queue` - Stream (training) queue generation
- `verbose_current_queue` - Queue generation with verbose output

**Key Metrics**:
- Queue generation time (target: <100ms for 30 songs)
- Memory usage during generation
- Strategy-specific performance characteristics

### 4. Path Translation Benchmarks

**Purpose**: Measure file path conversion performance between absolute and MPD-relative formats.

**Benchmarks**:
- `abs_to_mpd` - Absolute to MPD relative path conversion
- `mpd_to_abs` - MPD relative to absolute path conversion

**Key Metrics**:
- Conversion latency (target: <1ms)
- Path parsing efficiency
- Error handling overhead

### 5. Song Search Benchmarks

**Purpose**: Measure multi-strategy song search performance with realistic queries.

**Benchmarks**:
- Search performance with different query patterns:
  - Exact title matches
  - Artist searches
  - Album searches
  - Combined format searches
  - Fuzzy searches

**Key Metrics**:
- Search latency across different query types
- Database index utilization
- Pattern matching efficiency

### 6. Memory Operations Benchmarks

**Purpose**: Measure memory allocation and data structure performance.

**Benchmarks**:
- `large_song_vector` - Large data structure creation
- `string_operations` - Music metadata string processing
- `queue_operations` - Queue manipulation operations

**Key Metrics**:
- Memory allocation overhead
- String processing efficiency
- Collection operation performance

## Performance Targets

### Latency Targets

| Operation | Target Latency | Acceptable Range |
|-----------|----------------|------------------|
| Single song scoring | <100μs | 50μs - 200μs |
| Database song search | <5ms | 1ms - 10ms |
| Queue generation (30 songs) | <100ms | 50ms - 200ms |
| Path translation | <1ms | 100μs - 2ms |
| Stats update | <1ms | 500μs - 2ms |

### Throughput Targets

| Operation | Target Throughput | Minimum Acceptable |
|-----------|------------------|-------------------|
| Song scoring | >10,000 songs/sec | >5,000 songs/sec |
| Database updates | >1,000 updates/sec | >500 updates/sec |
| Search queries | >200 queries/sec | >100 queries/sec |

### Memory Targets

| Component | Target Memory | Maximum Acceptable |
|-----------|---------------|-------------------|
| Algorithm scoring | <1MB per 1000 songs | <5MB per 1000 songs |
| Queue generation | <10MB | <50MB |
| Database operations | <100MB | <500MB |

## Interpreting Results

### Understanding Criterion Output

```
algorithm_scoring/single_song_score
                        time:   [85.234 μs 87.123 μs 89.456 μs]
                        change: [-2.3% +0.1% +2.8%] (p = 0.89 > 0.05)
                        No change in performance detected.
```

**Key Elements**:
- **Time range**: [lower_bound mean upper_bound] - confidence interval
- **Change**: Performance change vs. previous baseline
- **P-value**: Statistical significance (p < 0.05 indicates significant change)

### Performance Regression Detection

Monitor these indicators for performance regression:

1. **Significant increase in mean latency** (>10% degradation)
2. **Increased variance** in timing measurements
3. **Memory usage growth** beyond target ranges
4. **Throughput reduction** below minimum thresholds

### Optimization Opportunities

Use benchmark results to identify:

- **Bottleneck operations** (highest latency/lowest throughput)
- **Scaling issues** (performance degradation with data size)
- **Memory inefficiencies** (excessive allocations)
- **Algorithm complexity** problems

## Continuous Integration

### Automated Benchmarking

Add to CI pipeline:

```yaml
# Example GitHub Actions benchmark job
benchmark:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v3
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Run benchmarks
      run: cargo bench --bench muse_benchmarks
    - name: Upload benchmark results
      uses: actions/upload-artifact@v3
      with:
        name: benchmark-results
        path: target/criterion/
```

### Performance Monitoring

1. **Baseline Establishment**: Create performance baselines for each release
2. **Regression Detection**: Alert on significant performance degradation
3. **Trend Analysis**: Monitor performance trends over time
4. **Platform Comparison**: Compare performance across different systems

## Hardware Considerations

### Recommended Test Environment

- **CPU**: Modern multi-core processor (4+ cores)
- **RAM**: 8GB+ for large dataset benchmarks
- **Storage**: SSD for database operation benchmarks
- **OS**: Linux (Ubuntu/Debian) for consistent results

### Platform-Specific Notes

- **Linux**: Most consistent benchmark results
- **macOS**: Generally good performance, may have filesystem differences
- **Windows**: Potential path separator differences in path translation benchmarks

## Troubleshooting

### Common Issues

1. **Inconsistent Results**
   - Solution: Run multiple iterations, check system load
   - Use `--sample-size` and `--measurement-time` parameters

2. **Database Benchmarks Failing**
   - Solution: Ensure sufficient disk space and write permissions
   - Check SQLite version compatibility

3. **Memory Benchmarks Unstable**
   - Solution: Close other applications, ensure sufficient RAM
   - Monitor system memory usage during benchmarks

### Debug Mode

Run benchmarks with additional debugging:

```bash
RUST_LOG=debug cargo bench
```

## Contributing Benchmark Improvements

### Adding New Benchmarks

1. Add benchmark function to `benches/muse_benchmarks.rs`
2. Follow naming convention: `benchmark_<category>_<operation>`
3. Include realistic test data
4. Add proper documentation
5. Update this document

### Benchmark Best Practices

1. **Use `black_box()`** to prevent compiler optimizations
2. **Realistic test data** - use representative dataset sizes
3. **Proper setup/teardown** - use `iter_batched()` for expensive setup
4. **Meaningful measurements** - focus on user-visible performance
5. **Statistical significance** - ensure sufficient sample sizes

## Performance Analysis Tools

### Additional Profiling

For deeper performance analysis:

```bash
# CPU profiling with perf
cargo bench --bench muse_benchmarks -- --profile-time=5

# Memory profiling with valgrind
cargo bench --bench muse_benchmarks --target-dir=/tmp/muse-bench

# Flame graph generation
cargo flamegraph --bench muse_benchmarks
```

### System Monitoring

Monitor system resources during benchmarks:

```bash
# CPU and memory usage
htop

# Disk I/O
iotop

# System call tracing
strace -c cargo bench
```

---

**Note**: Benchmark results are highly dependent on hardware, system load, and configuration. Always run benchmarks multiple times and compare results across consistent environments for meaningful analysis.