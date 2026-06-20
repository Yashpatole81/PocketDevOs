#!/bin/bash
# PocketDevOS Rust Installer for Termux
# Usage: curl -fsSL https://raw.githubusercontent.com/Yashpatole81/PocketDevOS/main/scripts/install.sh | bash

set -e

echo "╔══════════════════════════════════════════╗"
echo "║     Installing PocketDevOS (Rust)...     ║"
echo "╚══════════════════════════════════════════╝"

# Check if running in Termux
if [ ! -d "/data/data/com.termux" ] && [ -z "$PREFIX" ]; then
  echo "[!] Warning: Not running in Termux. Continuing anyway..."
fi

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
  echo "[*] Installing Rust..."
  pkg install -y rust || apt install -y rustc
fi

# Install git if not present
if ! command -v git &> /dev/null; then
  echo "[*] Installing git..."
  pkg install -y git || apt install -y git
fi

# Install build tools
pkg install -y make clang lld 2>/dev/null || apt install -y make clang 2>/dev/null || true

# Install Node.js (for frontend build)
if ! command -v node &> /dev/null; then
  echo "[*] Installing Node.js..."
  pkg install -y nodejs || apt install -y nodejs
fi

# Clone or update PocketDevOS
INSTALL_DIR="$HOME/.pocketdevos/app"
if [ -d "$INSTALL_DIR" ]; then
  echo "[*] Updating PocketDevOS..."
  cd "$INSTALL_DIR" && git pull
else
  echo "[*] Downloading PocketDevOS..."
  mkdir -p "$HOME/.pocketdevos"
  git clone --depth 1 https://github.com/Yashpatole81/PocketDevOS.git "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"

# Build Rust backend
echo "[*] Building Rust backend..."
cd backend
cargo build --release
cd ..

# Build frontend
echo "[*] Building frontend..."
cd frontend
npm install
npm run build
cd ..

# Create launcher script
LAUNCHER="$PREFIX/bin/pocketdevos"
if [ -z "$PREFIX" ]; then
  LAUNCHER="$HOME/.local/bin/pocketdevos"
  mkdir -p "$HOME/.local/bin"
fi

cat > "$LAUNCHER" << EOF
#!/bin/bash
INSTALL_DIR="$HOME/.pocketdevos/app"
cd "$INSTALL_DIR"
exec ./backend/target/release/pocketdevos "\$@"
EOF

chmod +x "$LAUNCHER"

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║     PocketDevOS installed!               ║"
echo "╠══════════════════════════════════════════╣"
echo "║  Run: pocketdevos                        ║"
echo "║  Then open browser to localhost:3000     ║"
echo "╚══════════════════════════════════════════╝"
