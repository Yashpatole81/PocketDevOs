import { useEffect, useState, useCallback } from 'react';
import { fs, type FsEntry } from '../../lib/api';
import { useAppStore } from '../../store/appStore';

export default function FileExplorer() {
  const currentPath = useAppStore(s => s.currentPath);
  const setCurrentPath = useAppStore(s => s.setCurrentPath);
  const openFile = useAppStore(s => s.openFile);

  const [entries, setEntries] = useState<FsEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadDirectory = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await fs.readdir(path);
      setEntries(result.items);
    } catch (err: any) {
      setError(err.message || 'Failed to load directory');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDirectory(currentPath);
  }, [currentPath, loadDirectory]);

  const handleEntryClick = (entry: FsEntry) => {
    if (entry.isDirectory) {
      setCurrentPath(entry.path);
    } else {
      // Open file in editor
      fs.read(entry.path).then(result => {
        openFile(entry.path, entry.name, result.content);
      }).catch(err => {
        console.error('Failed to open file:', err);
      });
    }
  };

  const goUp = () => {
    const parent = currentPath.split('/').slice(0, -1).join('/') || '/';
    setCurrentPath(parent);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Path bar */}
      <div className="flex items-center gap-2 h-10 px-3 border-b border-[var(--color-border)] bg-[var(--color-surface-1)]">
        <button
          onClick={goUp}
          disabled={currentPath === '/'}
          className="flex items-center justify-center w-8 h-8 rounded hover:bg-[var(--color-surface-2)] text-[var(--color-text-muted)] disabled:opacity-30 transition-colors"
        >
          ←
        </button>
        <div className="flex-1 text-xs text-[var(--color-text-secondary)] truncate font-mono">
          {currentPath}
        </div>
      </div>

      {/* File list */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center h-full text-[var(--color-text-muted)] text-sm">
            Loading...
          </div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center h-full p-4 text-center">
            <span className="text-[var(--color-danger)] text-sm mb-2">{error}</span>
            <button
              onClick={() => loadDirectory(currentPath)}
              className="text-xs px-3 py-1.5 rounded bg-[var(--color-surface-2)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] transition-colors"
            >
              Retry
            </button>
          </div>
        ) : entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-[var(--color-text-muted)]">
            <span className="text-2xl mb-2 opacity-30">📂</span>
            <span className="text-sm">Empty directory</span>
          </div>
        ) : (
          entries.map(entry => (
            <button
              key={entry.path}
              onClick={() => handleEntryClick(entry)}
              className="flex items-center gap-3 w-full px-4 py-3 hover:bg-[var(--color-surface-2)] transition-colors text-left border-b border-[var(--color-border)]/50"
            >
              <span className="text-lg shrink-0">
                {entry.isDirectory ? '📁' : getFileIcon(entry.name)}
              </span>
              <span className="text-sm text-[var(--color-text-primary)] truncate flex-1 min-w-0">
                {entry.name}
              </span>
              {entry.isSymlink && (
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-surface-3)] text-[var(--color-text-muted)] shrink-0">
                  symlink
                </span>
              )}
            </button>
          ))
        )}
      </div>
    </div>
  );
}

function getFileIcon(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'rs': return '🦀';
    case 'js': case 'ts': case 'jsx': case 'tsx': case 'mjs': return '📜';
    case 'py': return '🐍';
    case 'go': return '🐹';
    case 'md': case 'markdown': return '📝';
    case 'json': case 'yaml': case 'yml': case 'toml': return '⚙️';
    case 'html': case 'htm': return '🌐';
    case 'css': case 'scss': case 'less': return '🎨';
    case 'sh': case 'bash': case 'zsh': return '⌘';
    case 'dockerfile': return '🐳';
    case 'sql': return '🗃️';
    default: return '📄';
  }
}
