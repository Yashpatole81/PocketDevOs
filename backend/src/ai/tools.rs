use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

use crate::lib::security::WorkspaceGuard;

/// AI tool definitions and execution
pub struct AiTools {
    guard: Arc<WorkspaceGuard>,
}

impl AiTools {
    pub fn new(guard: Arc<WorkspaceGuard>) -> Self {
        Self { guard }
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "list_directory" => self.list_directory(args).await,
            "run_command" => self.run_command(args).await,
            "grep" => self.grep(args).await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn read_file(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or("path required")?;

        self.guard.validate(path).map_err(|e| e.to_string())?;

        // Check file size
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| e.to_string())?;

        if metadata.len() > 5 * 1024 * 1024 {
            return Err("File too large (max 5MB)".to_string());
        }

        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| e.to_string())
    }

    async fn write_file(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or("path required")?;
        let content = args
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        self.guard.validate(path).map_err(|e| e.to_string())?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| e.to_string())?;

        Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
    }

    async fn list_directory(&self, args: serde_json::Value) -> Result<String, String> {
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or("path required")?;

        self.guard.validate(path).map_err(|e| e.to_string())?;

        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| e.to_string())?;

        let mut result = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let ft = entry.file_type().await.unwrap_or_else(|_| {
                std::fs::metadata(entry.path())
                    .map(|m| m.file_type())
                    .unwrap_or_else(|_| std::fs::FileType::from(std::os::unix::fs::FileTypeExt::block_device()))
            });

            let icon = if ft.is_dir() { "📁" } else { "📄" };
            result.push(format!("{} {}", icon, name));
        }

        if result.is_empty() {
            Ok("(empty directory)".to_string())
        } else {
            Ok(result.join("\n"))
        }
    }

    async fn run_command(&self, args: serde_json::Value) -> Result<String, String> {
        let command = args
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or("command required")?;
        let cwd = args
            .get("cwd")
            .and_then(|c| c.as_str())
            .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_string()).as_str());

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        let output = timeout(
            Duration::from_secs(30),
            Command::new(&shell)
                .arg("-c")
                .arg(command)
                .current_dir(cwd)
                .env("TERM", "dumb")
                .output(),
        )
        .await;

        match output {
            Ok(Ok(output)) => {
                let mut result = String::new();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str("[stderr] ");
                    result.push_str(&stderr);
                }

                if result.is_empty() {
                    Ok("(no output)".to_string())
                } else {
                    Ok(result)
                }
            }
            Ok(Err(e)) => Err(format!("Command error: {}", e)),
            Err(_) => Err("Command timed out after 30s".to_string()),
        }
    }

    async fn grep(&self, args: serde_json::Value) -> Result<String, String> {
        let pattern = args
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or("pattern required")?;
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or("path required")?;
        let include = args.get("include").and_then(|i| i.as_str());

        self.guard.validate(path).map_err(|e| e.to_string())?;

        let mut cmd = Command::new("grep");
        cmd.arg("-rn").arg("--color=never");

        if let Some(glob) = include {
            cmd.arg(format!("--include={}", glob));
        }

        cmd.arg(pattern).arg(path);

        let output = timeout(Duration::from_secs(10), cmd.output()).await;

        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.trim().split('\n').collect();

                if lines.len() > 50 {
                    let head: Vec<&str> = lines.iter().take(50).copied().collect();
                    Ok(format!("{}\n... ({} more matches)", head.join("\n"), lines.len() - 50))
                } else {
                    Ok(stdout.trim().to_string())
                }
            }
            Ok(Err(_)) | Err(_) => Ok("No matches found".to_string()),
        }
    }

    /// Convert tools to OpenAI function schema
    pub fn to_openai_schema(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file at the given path.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Absolute path to the file to read"}
                        },
                        "required": ["path"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write content to a file. Creates if it doesn't exist, overwrites if it does.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Absolute path to the file to write"},
                            "content": {"type": "string", "description": "Content to write to the file"}
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_directory",
                    "description": "List the contents of a directory.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Absolute path to the directory to list"}
                        },
                        "required": ["path"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "run_command",
                    "description": "Execute a shell command. 30s timeout.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "command": {"type": "string", "description": "The shell command to execute"},
                            "cwd": {"type": "string", "description": "Working directory for the command"}
                        },
                        "required": ["command"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "grep",
                    "description": "Search for a pattern in files within a directory.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "pattern": {"type": "string"},
                            "path": {"type": "string"},
                            "include": {"type": "string", "description": "File glob pattern (e.g. '*.ts')"}
                        },
                        "required": ["pattern", "path"]
                    }
                }
            }),
        ]
    }
}
