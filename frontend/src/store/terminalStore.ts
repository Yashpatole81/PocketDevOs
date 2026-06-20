import { create } from 'zustand';

export interface TerminalSession {
  id: string;
  cols: number;
  rows: number;
  name: string;
}

interface TerminalState {
  sessions: TerminalSession[];
  activeSessionId: string | null;
  addSession: (session: TerminalSession) => void;
  removeSession: (id: string) => void;
  setActiveSession: (id: string) => void;
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
  sessions: [],
  activeSessionId: null,

  addSession: (session) => {
    const { sessions } = get();
    set({ sessions: [...sessions, session], activeSessionId: session.id });
  },

  removeSession: (id) => {
    const { sessions, activeSessionId } = get();
    const newSessions = sessions.filter(s => s.id !== id);
    const newActive = activeSessionId === id
      ? (newSessions.length > 0 ? newSessions[newSessions.length - 1].id : null)
      : activeSessionId;
    set({ sessions: newSessions, activeSessionId: newActive });
  },

  setActiveSession: (id) => set({ activeSessionId: id }),
}));
