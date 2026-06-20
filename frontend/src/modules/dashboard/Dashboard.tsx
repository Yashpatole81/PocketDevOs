import { useEffect, useState } from 'react';
import { useAppStore } from '../../store/appStore';

export default function Dashboard() {
  const setActiveTab = useAppStore(s => s.setActiveTab);
  const [workspace, setWorkspace] = useState<string>('');
  const [time, setTime] = useState(new Date());

  useEffect(() => {
    fetch('/api/workspace?token=' + (new URLSearchParams(window.location.search).get('token') || ''))
      .then(r => r.ok ? r.json() : { home: '~' })
      .then(d => setWorkspace(d.home));

    const interval = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const actions = [
    { tab: 'terminal', label: 'Open Terminal', desc: 'Access shell via native PTY', icon: '⌘' },
    { tab: 'editor', label: 'Open Editor', desc: 'Edit files with CodeMirror', icon: '📝' },
    { tab: 'explorer', label: 'File Explorer', desc: 'Browse and manage files', icon: '📁' },
    { tab: 'ai', label: 'AI Assistant', desc: 'Chat with local AI models', icon: '✦' },
  ];

  return (
    <div className="h-full overflow-y-auto p-4">
      {/* Status Card */}
      <div className="mb-4 p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border)]">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs uppercase tracking-wider text-[var(--color-text-muted)] font-medium">Status</span>
          <div className="flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-[var(--color-accent)] animate-pulse" />
            <span className="text-xs text-[var(--color-accent)]">Running</span>
          </div>
        </div>
        <div className="text-sm text-[var(--color-text-secondary)]">
          <div className="flex justify-between py-1">
            <span>Workspace</span>
            <span className="text-[var(--color-text-primary)] font-mono">{workspace}</span>
          </div>
          <div className="flex justify-between py-1">
            <span>Backend</span>
            <span className="text-[var(--color-text-primary)]">Rust (Axum + Tokio)</span>
          </div>
          <div className="flex justify-between py-1">
            <span>PTY</span>
            <span className="text-[var(--color-text-primary)]">Native (portable-pty)</span>
          </div>
          <div className="flex justify-between py-1">
            <span>Time</span>
            <span className="text-[var(--color-text-primary)] font-mono">{time.toLocaleTimeString()}</span>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <h2 className="text-xs uppercase tracking-wider text-[var(--color-text-muted)] font-medium mb-3 px-1">Quick Actions</h2>
      <div className="grid grid-cols-1 gap-2">
        {actions.map(a => (
          <button
            key={a.tab}
            onClick={() => setActiveTab(a.tab as any)}
            className="flex items-center gap-3 p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-all text-left active:scale-[0.98]"
          >
            <span className="text-2xl">{a.icon}</span>
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-[var(--color-text-primary)]">{a.label}</div>
              <div className="text-xs text-[var(--color-text-muted)]">{a.desc}</div>
            </div>
            <span className="text-[var(--color-text-muted)]">›</span>
          </button>
        ))}
      </div>

      {/* System Info */}
      <div className="mt-4 p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border)]">
        <span className="text-xs uppercase tracking-wider text-[var(--color-text-muted)] font-medium">About</span>
        <p className="text-xs text-[var(--color-text-secondary)] mt-2 leading-relaxed">
          PocketDevOS is an AI-native development workspace that runs on your Android device via Termux.
          The Rust backend provides a native PTY terminal, file system access, and AI chat via
          OpenAI-compatible APIs (Ollama, NVIDIA, or custom endpoints).
        </p>
      </div>
    </div>
  );
}
