import { useState, useRef, useEffect, useCallback } from 'react';
import { ai } from '../../lib/api';
import { useAIStore } from '../../store/aiStore';

export default function AIChat() {
  const { messages, isStreaming, streamingContent, addMessage, appendStreaming, finalizeStreaming, setStreaming, clear } = useAIStore();
  const [input, setInput] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef<AbortController | null>(null);

  // Auto-scroll
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, streamingContent]);

  const handleSend = useCallback(async () => {
    if (!input.trim() || isStreaming) return;

    const userMsg = input.trim();
    setInput('');

    addMessage({ id: Date.now().toString(), role: 'user', content: userMsg });
    setStreaming(true);

    try {
      const sessionId = 'session-' + Date.now();
      const history = [
        ...messages.map(m => ({ role: m.role, content: m.content })),
        { role: 'user', content: userMsg },
      ];

      const response = await ai.chat(history, sessionId);

      if (!response.ok || !response.body) {
        throw new Error(`HTTP ${response.status}`);
      }

      // Read SSE stream
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // Process complete SSE events
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (let i = 0; i < lines.length; i++) {
          const line = lines[i].trim();
          if (line.startsWith('data: ')) {
            const data = line.slice(6);
            if (data === '[DONE]') continue;

            try {
              const event = JSON.parse(data);
              if (event.type === 'text' && event.content) {
                appendStreaming(event.content);
              } else if (event.type === 'error') {
                addMessage({ id: Date.now().toString(), role: 'assistant', content: `Error: ${event.message || 'Unknown error'}` });
                setStreaming(false);
                return;
              } else if (event.type === 'done') {
                finalizeStreaming();
                return;
              }
            } catch {
              // Not JSON, might be raw text
              if (data) appendStreaming(data);
            }
          }
        }
      }

      finalizeStreaming();
    } catch (err: any) {
      console.error('Chat error:', err);
      addMessage({ id: Date.now().toString(), role: 'assistant', content: `Error: ${err.message}` });
      setStreaming(false);
    }
  }, [input, isStreaming, messages, addMessage, appendStreaming, finalizeStreaming, setStreaming]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between h-10 px-4 border-b border-[var(--color-border)] bg-[var(--color-surface-1)] shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">AI Assistant</span>
          {isStreaming && (
            <span className="flex items-center gap-1 text-[10px] text-[var(--color-accent)]">
              <span className="w-1.5 h-1.5 rounded-full bg-[var(--color-accent)] animate-pulse" />
              Thinking...
            </span>
          )}
        </div>
        <button
          onClick={clear}
          className="text-xs px-2 py-1 rounded text-[var(--color-text-muted)] hover:text-[var(--color-danger)] hover:bg-[var(--color-surface-2)] transition-colors"
        >
          Clear
        </button>
      </div>

      {/* Messages */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 && !streamingContent && (
          <div className="flex flex-col items-center justify-center h-full text-[var(--color-text-muted)]">
            <span className="text-3xl mb-3">✦</span>
            <p className="text-sm mb-1">How can I help?</p>
            <p className="text-xs text-center max-w-[280px]">
              Ask me to write code, explain concepts, or help with your project.
            </p>
          </div>
        )}

        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[85%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed ${
                msg.role === 'user'
                  ? 'bg-[var(--color-accent)] text-black rounded-br-md'
                  : 'bg-[var(--color-surface-2)] text-[var(--color-text-primary)] rounded-bl-md border border-[var(--color-border)]'
              }`}
            >
              <div className="whitespace-pre-wrap break-words">{msg.content}</div>
            </div>
          </div>
        ))}

        {streamingContent && (
          <div className="flex justify-start">
            <div className="max-w-[85%] rounded-2xl rounded-bl-md px-4 py-2.5 bg-[var(--color-surface-2)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] leading-relaxed">
              <div className="whitespace-pre-wrap break-words">{streamingContent}</div>
              <span className="inline-block w-1.5 h-4 ml-0.5 bg-[var(--color-accent)] animate-pulse align-middle" />
            </div>
          </div>
        )}
      </div>

      {/* Input */}
      <div className="shrink-0 p-3 border-t border-[var(--color-border)] bg-[var(--color-surface-1)]">
        <div className="flex items-end gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask anything..."
            rows={1}
            className="flex-1 min-h-[40px] max-h-[120px] px-3 py-2 rounded-xl bg-[var(--color-surface-2)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] resize-none focus:outline-none focus:border-[var(--color-accent)] transition-colors"
            style={{ fieldSizing: 'content' }}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || isStreaming}
            className="flex items-center justify-center w-10 h-10 rounded-xl bg-[var(--color-accent)] text-black disabled:opacity-30 disabled:cursor-not-allowed hover:bg-[var(--color-accent-hover)] transition-colors shrink-0"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="22" y1="2" x2="11" y2="13" />
              <polygon points="22 2 15 22 11 13 2 9 22 2" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
