use std::path::{Path, PathBuf};

/// Paths that should never be read or written
const DENIED_PATTERNS: &[&str] = &[
    ".env",
    ".ssh",
    ".gnupg",
    ".aws/credentials",
    ".npmrc",
    ".netrc",
    "id_rsa",
    "id_ed25519",
    ".pocketdevos/keys",
    "/etc/shadow",
    "/etc/passwd",
];

/// Check if a path is denied (sensitive file)
pub fn is_denied_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    DENIED_PATTERNS.iter().any(|pattern| normalized.contains(pattern))
}

/// Workspace root authorization
pub struct WorkspaceGuard {
    roots: Vec<PathBuf>,
}

impl WorkspaceGuard {
    pub fn new() -> Self {
        let mut roots = Vec::new();
        // Always authorize home directory
        if let Ok(home) = std::env::var("HOME") {
            roots.push(PathBuf::from(home));
        } else if let Ok(profile) = std::env::var("USERPROFILE") {
            roots.push(PathBuf::from(profile));
        } else {
            roots.push(PathBuf::from("."));
        }
        // On Termux, also allow Termux prefix
        if let Ok(prefix) = std::env::var("PREFIX") {
            roots.push(PathBuf::from(prefix));
        }
        Self { roots }
    }

    pub fn authorize(&mut self, path: &str) {
        self.roots.push(PathBuf::from(path));
    }

    pub fn is_authorized(&self, target_path: &str) -> bool {
        let target = PathBuf::from(target_path);
        let target_canonical = target.canonicalize().unwrap_or(target);

        for root in &self.roots {
            let root_canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
            if target_canonical.starts_with(&root_canonical) {
                return true;
            }
        }
        false
    }

    /// Validate a path: must be authorized AND not denied
    pub fn validate(&self, path: &str) -> Result<(), String> {
        if is_denied_path(path) {
            return Err("Access denied: sensitive path".to_string());
        }
        if !self.is_authorized(path) {
            return Err("Access denied: outside workspace".to_string());
        }
        Ok(())
    }
}

impl Default for WorkspaceGuard {
    fn default() -> Self {
        Self::new()
    }
}
