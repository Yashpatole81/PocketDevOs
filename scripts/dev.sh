#!/bin/bash
# Development script - starts backend and frontend dev server
# Usage: ./scripts/dev.sh

set -e

echo "[*] Starting PocketDevOS in development mode..."

# Start Rust backend in background
echo "[*] Starting Rust backend..."
cd backend
cargo run &
BACKEND_PID=$!
cd ..

# Wait for backend
sleep 3

# Start frontend dev server
echo "[*] Starting frontend dev server..."
cd frontend
npm install
npm run dev &
FRONTEND_PID=$!
cd ..

echo ""
echo "[*] Both services running!"
echo "[*] Frontend: http://localhost:5173"
echo "[*] Backend:  http://localhost:3000"
echo ""
echo "[*] Press Ctrl+C to stop both"

# Trap to kill both on exit
trap "kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" INT TERM
wait
