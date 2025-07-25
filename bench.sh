#!/bin/bash
# Muse Benchmark Runner
# 
# This script runs comprehensive performance benchmarks for the Muse music player
# and generates detailed reports.

set -e

echo "🎵 Muse Performance Benchmark Suite"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create reports directory
REPORTS_DIR="target/criterion"
mkdir -p "$REPORTS_DIR"

echo -e "${BLUE}Building optimized release version...${NC}"
cargo build --release

echo -e "${BLUE}Running comprehensive benchmarks...${NC}"

# Run different benchmark categories
echo -e "${YELLOW}1. Algorithm Performance Benchmarks${NC}"
cargo bench --bench muse_benchmarks algorithm_scoring

echo -e "${YELLOW}2. Database Operations Benchmarks${NC}"
cargo bench --bench muse_benchmarks database_operations

echo -e "${YELLOW}3. Queue Generation Benchmarks${NC}"
cargo bench --bench muse_benchmarks queue_generation

echo -e "${YELLOW}4. Path Translation Benchmarks${NC}"
cargo bench --bench muse_benchmarks path_translation

echo -e "${YELLOW}5. Song Search Benchmarks${NC}"
cargo bench --bench muse_benchmarks song_search

echo -e "${YELLOW}6. Memory Operations Benchmarks${NC}"
cargo bench --bench muse_benchmarks memory_operations

echo -e "${GREEN}✅ All benchmarks completed!${NC}"

# Check if HTML reports were generated
if [ -d "$REPORTS_DIR" ]; then
    echo -e "${BLUE}📊 Benchmark reports generated in: $REPORTS_DIR${NC}"
    echo -e "${BLUE}📈 View HTML reports by opening: $REPORTS_DIR/muse_benchmarks/report/index.html${NC}"
else
    echo -e "${YELLOW}⚠️  HTML reports not found. Install gnuplot for graphical reports.${NC}"
fi

echo -e "${GREEN}🎯 Benchmark Summary:${NC}"
echo "• Algorithm scoring: Individual song scoring, batch processing, ranking"
echo "• Database ops: Query performance, inserts, updates, connection lookups"  
echo "• Queue generation: Current, Thread, Stream queue strategies"
echo "• Path translation: File path conversion between formats"
echo "• Song search: Multi-strategy search with different query patterns"
echo "• Memory ops: Large data structures, string operations, vector manipulations"

echo -e "${BLUE}💡 Tips:${NC}"
echo "• Run 'cargo bench' for quick benchmarks"
echo "• Use 'cargo bench -- --help' for more options"
echo "• Compare results across different systems and configurations"
echo "• Monitor performance regression with CI/CD integration"

echo -e "${GREEN}🎵 Happy benchmarking!${NC}"