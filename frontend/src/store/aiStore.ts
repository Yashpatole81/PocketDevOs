import { create } from 'zustand';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  isStreaming?: boolean;
}

interface AIState {
  messages: ChatMessage[];
  isStreaming: boolean;
  streamingContent: string;
  addMessage: (msg: ChatMessage) => void;
  appendStreaming: (content: string) => void;
  finalizeStreaming: () => void;
  setStreaming: (v: boolean) => void;
  clear: () => void;
}

export const useAIStore = create<AIState>((set, get) => ({
  messages: [],
  isStreaming: false,
  streamingContent: '',

  addMessage: (msg) => {
    set({ messages: [...get().messages, msg] });
  },

  appendStreaming: (content) => {
    set({ streamingContent: get().streamingContent + content });
  },

  finalizeStreaming: () => {
    const { messages, streamingContent } = get();
    if (streamingContent) {
      set({
        messages: [...messages, { id: Date.now().toString(), role: 'assistant', content: streamingContent }],
        streamingContent: '',
        isStreaming: false,
      });
    }
  },

  setStreaming: (v) => set({ isStreaming: v }),

  clear: () => set({ messages: [], streamingContent: '', isStreaming: false }),
}));
