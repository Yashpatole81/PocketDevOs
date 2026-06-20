import { create } from 'zustand';

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  dirty: boolean;
}

type Tab = 'dashboard' | 'editor' | 'terminal' | 'explorer' | 'ai' | 'settings';

interface AppState {
  activeTab: Tab;
  setActiveTab: (tab: Tab) => void;

  openFiles: OpenFile[];
  activeFileIndex: number;
  openFile: (path: string, name: string, content: string) => void;
  setActiveFile: (index: number) => void;
  closeFile: (path: string) => void;
  updateFileContent: (path: string, content: string) => void;
  markFileSaved: (path: string) => void;

  currentPath: string;
  setCurrentPath: (path: string) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  activeTab: 'dashboard',
  setActiveTab: (tab) => set({ activeTab: tab }),

  openFiles: [],
  activeFileIndex: -1,

  openFile: (path, name, content) => {
    const { openFiles, activeFileIndex } = get();
    const existing = openFiles.findIndex(f => f.path === path);
    if (existing >= 0) {
      set({ activeFileIndex: existing, activeTab: 'editor' });
    } else {
      const newFiles = [...openFiles, { path, name, content, dirty: false }];
      set({ openFiles: newFiles, activeFileIndex: newFiles.length - 1, activeTab: 'editor' });
    }
  },

  setActiveFile: (index) => set({ activeFileIndex: index }),

  closeFile: (path) => {
    const { openFiles, activeFileIndex } = get();
    const idx = openFiles.findIndex(f => f.path === path);
    if (idx < 0) return;
    const newFiles = openFiles.filter((_, i) => i !== idx);
    let newIndex = activeFileIndex;
    if (idx === activeFileIndex) {
      newIndex = newFiles.length > 0 ? Math.min(idx, newFiles.length - 1) : -1;
    } else if (idx < activeFileIndex) {
      newIndex = activeFileIndex - 1;
    }
    set({ openFiles: newFiles, activeFileIndex: newIndex });
  },

  updateFileContent: (path, content) => {
    const { openFiles } = get();
    set({
      openFiles: openFiles.map(f =>
        f.path === path ? { ...f, content, dirty: f.content !== content } : f
      ),
    });
  },

  markFileSaved: (path) => {
    const { openFiles } = get();
    set({
      openFiles: openFiles.map(f =>
        f.path === path ? { ...f, dirty: false } : f
      ),
    });
  },

  currentPath: '/',
  setCurrentPath: (path) => set({ currentPath: path }),
}));
