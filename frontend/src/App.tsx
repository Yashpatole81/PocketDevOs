import { useEffect } from 'react';
import { initAuth } from './lib/api';
import { useAppStore } from './store/appStore';
import Terminal from './modules/terminal/Terminal';
import Editor from './modules/editor/Editor';
import FileExplorer from './modules/explorer/FileExplorer';
import AIChat from './modules/ai/AIChat';
import Dashboard from './modules/dashboard/Dashboard';

function App() {
  const activeTab = useAppStore(s => s.activeTab);
  const setActiveTab = useAppStore(s => s.setActiveTab);

  useEffect(() => {
    initAuth();
  }, []);

  return (
    <div className="h-full flex flex-col bg-[var(--color-bg)] text-[var(--color-text-primary)]">
      {/* Top Bar */}
      <header className="flex items-center justify-between px-4 h-12 border-b border-[var(--color-border)] bg-[var(--color-surface-1)] shrink-0">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded bg-[var(--color-accent)] flex items-center justify-center text-xs font-bold text-black">P</div>
          <span className="text-sm font-semibold tracking-tight">PocketDevOS</span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-text-muted)]">v0.2.0</span>
        </div>
        <div className="text-xs text-[var(--color-text-muted)]">Rust Backend</div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        {activeTab === 'dashboard' && <Dashboard />}
        {activeTab === 'terminal' && <Terminal />}
        {activeTab === 'editor' && <Editor />}
        {activeTab === 'explorer' && <FileExplorer />}
        {activeTab === 'ai' && <AIChat />}
      </main>

      {/* Bottom Navigation */}
      <nav className="flex items-center justify-around h-14 border-t border-[var(--color-border)] bg-[var(--color-surface-1)] shrink-0">
        <NavButton tab="dashboard" active={activeTab} icon="◈" label="Home" onClick={setActiveTab} />
        <NavButton tab="explorer" active={activeTab} icon="📁" label="Files" onClick={setActiveTab} />
        <NavButton tab="editor" active={activeTab} icon="📝" label="Editor" onClick={setActiveTab} />
        <NavButton tab="terminal" active={activeTab} icon="⌘" label="Terminal" onClick={setActiveTab} />
        <NavButton tab="ai" active={activeTab} icon="✦" label="AI" onClick={setActiveTab} />
      </nav>
    </div>
  );
}

function NavButton({ tab, active, icon, label, onClick }: {
  tab: string; active: string; icon: string; label: string; onClick: (t: any) => void;
}) {
  const isActive = active === tab;
  return (
    <button
      onClick={() => onClick(tab)}
      className={`flex flex-col items-center justify-center gap-0.5 w-16 h-full transition-colors ${
        isActive ? 'text-[var(--color-accent)]' : 'text-[var(--color-text-muted)]'
      }`}
    >
      <span className="text-base">{icon}</span>
      <span className="text-[10px] font-medium">{label}</span>
    </button>
  );
}

export default App;
