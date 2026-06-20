use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Information about a terminal session
#[derive(Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub cols: u16,
    pub rows: u16,
    pub created_at: u64,
}

/// Internal handle for a running session
pub struct SessionHandle {
    pub info: SessionInfo,
    pub stdin_tx: mpsc::Sender<String>,
    pub stdout_tx: broadcast::Sender<String>,
    pub exit_tx: broadcast::Sender<i32>,
    #[allow(dead_code)]
    pub task_handle: JoinHandle<()>,
}

/// Manages all PTY sessions using native PTYs (no `script` hack)
pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<String, SessionHandle>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new terminal session with a real native PTY
    pub async fn create(
        &self,
        cols: Option<u16>,
        rows: Option<u16>,
        cwd: Option<String>,
    ) -> Result<SessionInfo, String> {
        let id = Uuid::new_v4().to_string();
        let cols = cols.unwrap_or(80);
        let rows = rows.unwrap_or(24);
        let cwd = cwd.unwrap_or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string())
        });
        let shell = Self::detect_shell();

        info!("Creating PTY session {}: {}x{} cwd={} shell={}", id, cols, rows, cwd, shell);

        // Open a native PTY
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        // Build shell command
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("COLUMNS", cols.to_string());
        cmd.env("LINES", rows.to_string());
        cmd.env("POCKETDEVOS", "1");

        // Spawn shell in PTY
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        drop(pair.slave);

        // Communication channels
        let (stdin_tx, stdin_rx) = mpsc::channel::<String>(256);
        let (stdout_tx, _stdout_rx) = broadcast::channel::<String>(1024);
        let (exit_tx, _exit_rx) = broadcast::channel::<i32>(4);

        let stdout_tx_task = stdout_tx.clone();
        let exit_tx_task = exit_tx.clone();
        let id_for_task = id.clone();

        // Convert tokio mpsc to std mpsc for the blocking context
        let (std_stdin_tx, std_stdin_rx) = std::sync::mpsc::channel::<String>();

        // Spawn a bridge task: tokio mpsc -> std mpsc
        tokio::spawn(async move {
            let mut stdin_rx = stdin_rx;
            while let Some(data) = stdin_rx.recv().await {
                if std_stdin_tx.send(data).is_err() {
                    break;
                }
            }
        });

        // Spawn blocking task for PTY I/O
        let task_handle: JoinHandle<()> = tokio::task::spawn_blocking(move || {
            let mut master = pair.master;

            // Take writer for stdin
            let mut writer = match master.take_writer() {
                Ok(w) => w,
                Err(e) => {
                    error!("[{}] Failed to get PTY writer: {}", id_for_task, e);
                    return;
                }
            };

            // Take reader for stdout
            let mut reader = match master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    error!("[{}] Failed to clone PTY reader: {}", id_for_task, e);
                    return;
                }
            };

            // Stdin thread: std mpsc -> PTY writer
            let id_stdin = id_for_task.clone();
            std::thread::spawn(move || {
                while let Ok(data) = std_stdin_rx.recv() {
                    if writer.write_all(data.as_bytes()).is_err() {
                        warn!("[{}] PTY stdin write failed", id_stdin);
                        break;
                    }
                    if writer.flush().is_err() {
                        break;
                    }
                }
            });

            // Stdout loop: PTY reader -> broadcast
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        info!("[{}] PTY EOF", id_for_task);
                        break;
                    }
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        if stdout_tx_task.send(text).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        warn!("[{}] PTY read error: {}", id_for_task, e);
                        break;
                    }
                }
            }

            // Wait for child process
            let status = child.wait().ok();
            let code = status
                .map(|s| s.exit_code() as i32)
                .unwrap_or(1);
            let _ = exit_tx_task.send(code);
            info!("[{}] PTY process exited with code {}", id_for_task, code);
        });

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let info = SessionInfo {
            id: id.clone(),
            cols,
            rows,
            created_at,
        };

        let handle = SessionHandle {
            info: info.clone(),
            stdin_tx,
            stdout_tx,
            exit_tx,
            task_handle,
        };

        self.sessions.write().await.insert(id, handle);
        Ok(info)
    }

    /// Write data to a session's PTY stdin
    pub async fn write(&self, id: &str, data: &str) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| "Session not found".to_string())?;

        session
            .stdin_tx
            .send(data.to_string())
            .await
            .map_err(|_| "Session closed".to_string())
    }

    /// Get a session's stdout broadcast sender
    pub async fn get_stdout_tx(
        &self,
        id: &str,
    ) -> Result<broadcast::Sender<String>, String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| "Session not found".to_string())?;
        Ok(session.stdout_tx.clone())
    }

    /// Get a session's exit broadcast sender
    pub async fn get_exit_tx(&self, id: &str) -> Result<broadcast::Sender<i32>, String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| "Session not found".to_string())?;
        Ok(session.exit_tx.clone())
    }

    /// Get session info
    pub async fn get_info(&self, id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(id).map(|h| h.info.clone())
    }

    /// Kill a session
    pub async fn kill(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(id).is_some() {
            info!("Killed session {}", id);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// List all active sessions
    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|h| h.info.clone()).collect()
    }

    /// Detect the default shell
    fn detect_shell() -> String {
        // Check SHELL env var first (Unix/Termux)
        if let Ok(shell) = std::env::var("SHELL") {
            return shell;
        }

        // Windows: use PowerShell or cmd
        #[cfg(windows)]
        {
            if let Ok(comspec) = std::env::var("COMSPEC") {
                return comspec;
            }
            return "cmd.exe".to_string();
        }

        // Unix fallback
        #[cfg(not(windows))]
        {
            if std::path::Path::new("/data/data/com.termux/files/usr/bin/bash").exists() {
                "/data/data/com.termux/files/usr/bin/bash".to_string()
            } else {
                "/bin/bash".to_string()
            }
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}
