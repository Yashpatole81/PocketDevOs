import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';
import { terminal as terminalApi } from '../../lib/api';
import { useTerminalStore } from '../../store/terminalStore';

export default function Terminal() {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<XTerm | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  const { sessions, activeSessionId, addSession, removeSession, setActiveSession } = useTerminalStore();
  const [isConnecting, setIsConnecting] = useState(false);

  // Create terminal
  const createTerminal = useCallback(async () => {
    if (!containerRef.current || isConnecting) return;
    setIsConnecting(true);

    try {
      // Dispose existing
      if (termRef.current) {
        termRef.current.dispose();
        termRef.current = null;
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      // Create session
      const { id, cols, rows } = await terminalApi.create();

      // Setup xterm.js
      const term = new XTerm({
        cursorBlink: true,
        cursorStyle: 'bar',
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'SF Mono', 'Fira Code', 'Consolas', monospace",
        lineHeight: 1.3,
        theme: {
          background: '#0a0a0a',
          foreground: '#e0e0e0',
          cursor: '#ffffff',
          selectionBackground: 'rgba(74,222,128,0.2)',
          black: '#1a1a1a',
          red: '#f87171',
          green: '#4ade80',
          yellow: '#fbbf24',
          blue: '#60a5fa',
          magenta: '#c084fc',
          cyan: '#67e8f9',
          white: '#f4f4f4',
          brightBlack: '#666666',
          brightRed: '#fca5a5',
          brightGreen: '#86efac',
          brightYellow: '#fde68a',
          brightBlue: '#93c5fd',
          brightMagenta: '#d8b4fe',
          brightCyan: '#a5f3fc',
          brightWhite: '#ffffff',
        },
        allowProposedApi: true,
      });

      const fitAddon = new FitAddon();
      const webLinksAddon = new WebLinksAddon();

      term.loadAddon(fitAddon);
      term.loadAddon(webLinksAddon);

      term.open(containerRef.current);
      fitAddon.fit();

      termRef.current = term;
      fitRef.current = fitAddon;

      // Connect WebSocket
      const ws = terminalApi.connect(id);
      wsRef.current = ws;

      ws.onopen = () => {
        // Send initial size
        ws.send(JSON.stringify({ type: 'resize', cols: term.cols, rows: term.rows }));
      };

      ws.onmessage = (event) => {
        term.write(event.data);
      };

      ws.onclose = () => {
        term.write('\r\n\x1b[90m[Connection closed]\x1b[0m\r\n');
      };

      ws.onerror = () => {
        term.write('\r\n\x1b[31m[Connection error]\x1b[0m\r\n');
      };

      // User input -> WebSocket
      term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(data);
        }
      });

      // Resize handling
      const resizeObserver = new ResizeObserver(() => {
        fitAddon.fit();
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: 'resize',
            cols: term.cols,
            rows: term.rows,
          }));
        }
      });
      resizeObserver.observe(containerRef.current);

      addSession({ id, cols, rows, name: `Terminal ${sessions.length + 1}` });

      // Cleanup function
      return () => {
        resizeObserver.disconnect();
        ws.close();
        term.dispose();
      };
    } catch (err) {
      console.error('Failed to create terminal:', err);
    } finally {
      setIsConnecting(false);
    }
  }, [isConnecting, sessions.length, addSession]);

  // Create terminal on mount
  useEffect(() => {
    let cleanup: (() => void) | undefined;

    createTerminal().then((fn) => {
      cleanup = fn;
    });

    return () => {
      if (cleanup) cleanup();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="h-full flex flex-col">
      {/* Terminal Tabs */}
      <div className="flex items-center h-10 border-b border-[var(--color-border)] bg-[var(--color-surface-1)] overflow-x-auto">
        {sessions.map(s => (
          <div
            key={s.id}
            onClick={() => setActiveSession(s.id)}
            className={`flex items-center gap-2 px-3 h-full border-r border-[var(--color-border)] shrink-0 cursor-pointer transition-colors ${
              activeSessionId === s.id
                ? 'bg-[var(--color-surface-2)] text-[var(--color-text-primary)]'
                : 'text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]'
            }`}
          >
            <span className="text-xs truncate max-w-[120px]">{s.name}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                terminalApi.kill(s.id);
                removeSession(s.id);
              }}
              className="text-[10px] w-4 h-4 flex items-center justify-center rounded hover:bg-[var(--color-surface-3)]"
            >
              ×
            </button>
          </div>
        ))}
        <button
          onClick={createTerminal}
          disabled={isConnecting}
          className="flex items-center justify-center w-10 h-full border-r border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-accent)] transition-colors disabled:opacity-50"
        >
          +
        </button>
      </div>

      {/* Terminal Container */}
      <div ref={containerRef} className="flex-1 p-2" />
    </div>
  );
}
