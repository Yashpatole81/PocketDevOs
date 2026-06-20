import { useCallback, useMemo, useState, useEffect, useRef } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { tokyoNight } from '@uiw/codemirror-theme-tokyo-night';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { json } from '@codemirror/lang-json';
import { html } from '@codemirror/lang-html';
import { css } from '@codemirror/lang-css';
import { markdown } from '@codemirror/lang-markdown';
import { keymap } from '@codemirror/view';
import type { Extension } from '@codemirror/state';
import { useAppStore } from '../../store/appStore';
import { fs } from '../../lib/api';

function getLanguageExtension(path: string): Extension {
  const ext = path.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts': case 'tsx': case 'js': case 'jsx': case 'mjs': case 'cjs':
      return javascript({ jsx: true, typescript: ext === 'ts' || ext === 'tsx' });
    case 'py': return python();
    case 'json': return json();
    case 'html': case 'htm': return html();
    case 'css': case 'scss': return css();
    case 'md': case 'markdown': return markdown();
    default: return [];
  }
}

export default function Editor() {
  const openFiles = useAppStore(s => s.openFiles);
  const activeFileIndex = useAppStore(s => s.activeFileIndex);
  const setActiveFile = useAppStore(s => s.setActiveFile);
  const closeFile = useAppStore(s => s.closeFile);
  const updateFileContent = useAppStore(s => s.updateFileContent);

  const activeFile = activeFileIndex >= 0 ? openFiles[activeFileIndex] : null;
  const [saving, setSaving] = useState(false);

  const handleSave = useCallback(async () => {
    if (!activeFile) return;
    setSaving(true);
    try {
      await fs.write(activeFile.path, activeFile.content);
      useAppStore.getState().markFileSaved(activeFile.path);
    } catch (err) {
      console.error('Save failed:', err);
    } finally {
      setSaving(false);
    }
  }, [activeFile]);

  const saveKeymap = useMemo(() =>
    keymap.of([{
      key: 'Mod-s',
      run: () => { handleSave(); return true; },
    }]), [handleSave]);

  if (!activeFile) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-[var(--color-text-secondary)] p-4">
        <span className="text-4xl mb-4 opacity-30">📝</span>
        <p className="text-sm mb-2">No file open</p>
        <p className="text-xs text-[var(--color-text-muted)] text-center">
          Open a file from the Explorer to start editing
        </p>
      </div>
    );
  }

  const langExtension = getLanguageExtension(activeFile.path);
  const extensions = useMemo(() => [langExtension, saveKeymap], [langExtension, saveKeymap]);

  return (
    <div className="h-full flex flex-col">
      {/* Tabs */}
      <div className="flex items-center h-10 border-b border-[var(--color-border)] bg-[var(--color-surface-1)] overflow-x-auto">
        {openFiles.map((file, idx) => (
          <button
            key={file.path}
            onClick={() => setActiveFile(idx)}
            className={`flex items-center gap-2 px-3 h-full border-r border-[var(--color-border)] shrink-0 transition-colors ${
              idx === activeFileIndex
                ? 'bg-[var(--color-surface-2)] text-[var(--color-text-primary)]'
                : 'text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]'
            }`}
            style={{ minWidth: '80px' }}
          >
            <span className="text-xs truncate max-w-[120px]">
              {file.name}
              {file.dirty && <span className="ml-1 text-[var(--color-accent)]">●</span>}
            </span>
            <span
              onClick={(e) => { e.stopPropagation(); closeFile(file.path); }}
              className="ml-auto flex items-center justify-center w-5 h-5 rounded hover:bg-[var(--color-surface-3)] text-[var(--color-text-muted)]"
            >
              ×
            </span>
          </button>
        ))}
      </div>

      {/* Editor */}
      <div className="flex-1 overflow-hidden">
        <CodeMirror
          value={activeFile.content}
          height="100%"
          theme={tokyoNight}
          extensions={extensions}
          onChange={(value) => updateFileContent(activeFile.path, value)}
          basicSetup={{
            lineNumbers: true,
            highlightActiveLineGutter: true,
            highlightActiveLine: true,
            foldGutter: true,
            bracketMatching: true,
            closeBrackets: true,
            indentOnInput: true,
            tabSize: 2,
          }}
          className="h-full text-sm"
        />
      </div>

      {/* Status bar */}
      <div className="flex items-center justify-between h-7 px-3 border-t border-[var(--color-border)] bg-[var(--color-surface-1)] text-[10px] text-[var(--color-text-muted)]">
        <span>{activeFile.name}</span>
        <div className="flex items-center gap-3">
          {saving && <span className="text-[var(--color-accent)]">Saving...</span>}
          {activeFile.dirty && <span className="text-[var(--color-warning)]">Modified</span>}
          <button
            onClick={handleSave}
            className="px-2 py-0.5 rounded bg-[var(--color-accent)] text-black text-[10px] font-medium hover:bg-[var(--color-accent-hover)] transition-colors"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
