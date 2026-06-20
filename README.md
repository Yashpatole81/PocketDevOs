# PocketDevOS v0.2.0

AI-native development workspace for Android. Runs on Termux, accessed via browser.

## What's New in v0.2.0

- **Rust Backend**: Complete rewrite from Node.js to Rust (Axum + Tokio)
- **Native PTY**: Uses `portable-pty` instead of the `script` command hack
- **Better Performance**: ~10MB RAM usage vs ~100MB with Node.js
- **Single Binary**: No `npm install` needed on Termux — just one executable
- **React Frontend**: Modern React 19 + Vite + Tailwind CSS

## Architecture

```
PocketDevOS/
├── backend/           # Rust backend (Axum + Tokio + portable-pty)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # HTTP server, static files, middleware
│       ├── pty.rs            # Native PTY session manager
│       ├── routes/
│       │   ├── terminal.rs   # WebSocket PTY + session CRUD
│       │   ├── fs.rs         # File system CRUD
│       │   ├── shell.rs      # One-shot command execution
│       │   └── ai.rs         # SSE streaming AI chat
│       ├── ai/
│       │   ├── client.rs     # OpenAI-compatible HTTP client
│       │   └── tools.rs      # AI tool definitions (read_file, write_file, etc.)
│       └── lib/
│           ├── auth.rs       # Token-based auth middleware
│           └── security.rs   # Path deny-list + workspace guard
├── frontend/          # React 19 frontend (Vite + Tailwind)
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── App.tsx
│       ├── main.tsx
│       ├── lib/api.ts        # REST/WS client
│       ├── store/            # Zustand stores
│       └── modules/
│           ├── dashboard/
│           ├── terminal/
│           ├── editor/
│           ├── explorer/
│           └── ai/
└── scripts/
    ├── install.sh            # One-line Termux installer
    ├── build.sh              # Build both backend and frontend
    └── dev.sh                # Start dev mode (backend + frontend)
```

## Quick Start (Termux)

```bash
# Install in one command
curl -fsSL https://raw.githubusercontent.com/Yashpatole81/PocketDevOS/main/scripts/install.sh | bash

# Run
pocketdevos
```

Open `http://localhost:3000` in your browser.

## Quick Start (Development)

### Prerequisites
- Rust (latest stable)
- Node.js 20+

### Build & Run

```bash
git clone https://github.com/Yashpatole81/PocketDevOS.git
cd PocketDevOS

# Build everything
./scripts/build.sh

# Run the backend
./backend/target/release/pocketdevos

# Or use the dev script (starts both backend and frontend):
./scripts/dev.sh
```

## Manual Build

### Backend only
```bash
cd backend
cargo build --release
./target/release/pocketdevos
```

### Frontend only
```bash
cd frontend
npm install
npm run build
```

The frontend build output goes to `frontend/dist/`, which the Rust backend serves automatically.

## AI Configuration

The backend reads AI configuration from environment variables:

| Variable | Default | Description |
|---|---|---|
| `AI_BASE_URL` | `http://localhost:11434/v1` | OpenAI-compatible API endpoint |
| `AI_API_KEY` | `ollama` | API key (not needed for Ollama) |
| `AI_MODEL` | `qwen2.5-coder:7b` | Model identifier |
| `PORT` | `3000` | Server port |

### Supported Providers
- **Ollama** (local): Set `AI_BASE_URL=http://localhost:11434/v1`
- **NVIDIA Build**: Set `AI_BASE_URL=https://integrate.api.nvidia.com/v1` and `AI_API_KEY=your_key`
- **Custom**: Any OpenAI-compatible endpoint

## API Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/api/health` | Health check (no auth) |
| GET | `/api/workspace` | Get home directory |
| POST | `/api/terminal/create` | Create PTY session |
| GET | `/api/terminal/list` | List sessions |
| DELETE | `/api/terminal/:id` | Kill session |
| GET | `/api/terminal/:id/ws` | WebSocket for PTY I/O |
| GET | `/api/fs/readdir` | List directory |
| GET | `/api/fs/read` | Read file |
| POST | `/api/fs/write` | Write file |
| POST | `/api/fs/create` | Create file/dir |
| POST | `/api/fs/rename` | Rename file |
| POST | `/api/fs/delete` | Delete file |
| POST | `/api/shell/run` | Run one-shot command |
| GET | `/api/ai/config` | Get AI config |
| POST | `/api/ai/config` | Set AI config |
| POST | `/api/ai/chat` | Chat with SSE stream |
| POST | `/api/ai/stop` | Stop streaming |

All API endpoints (except `/api/health`) require Bearer token auth. The token is printed on server startup.

## Tech Stack

**Backend**: Rust, Axum, Tokio, portable-pty, reqwest, serde  
**Frontend**: React 19, Vite, xterm.js, CodeMirror 6, Tailwind CSS, Zustand  
**AI**: OpenAI-compatible APIs (Ollama, NVIDIA Build, custom)  

## License

Apache-2.0
