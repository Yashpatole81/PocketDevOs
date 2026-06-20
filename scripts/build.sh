#!/bin/bash
# Build script for PocketDevOS
# Usage: ./scripts/build.sh

set -e

echo "[*] Building PocketDevOS..."

# Build Rust backend
echo "[*] Building Rust backend (release)..."
cd backend
cargo build --release
cd ..

# Build frontend
echo "[*] Building frontend..."
cd frontend
npm install
npm run build
cd ..

echo ""
echo "[*] Build complete!"
echo "[*] Run: ./backend/target/release/pocketdevos"
echo "[*] Or:  cargo run --release -p pocketdevos (from backend/)"
