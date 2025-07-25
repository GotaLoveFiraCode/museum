#!/bin/bash

# Build script for Muse v2

echo "Building Muse v2..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build in release mode
cargo build --release

if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo "Binary is at: target/release/muse"
else
    echo "Build failed!"
    exit 1
fi