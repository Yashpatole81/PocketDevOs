/**
 * API client for communicating with the PocketDevOS Rust backend.
 */

const BASE_URL = "";

let authToken: string | null = null;

export function initAuth(): void {
  const url = new URL(window.location.href);
  const token = url.searchParams.get("token");
  if (token) {
    authToken = token;
    url.searchParams.delete("token");
    window.history.replaceState({}, "", url.toString());
  }
}

export function setAuthToken(token: string): void {
  authToken = token;
}

export function getAuthToken(): string | null {
  return authToken;
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };
  if (authToken) {
    headers["Authorization"] = `Bearer ${authToken}`;
  }
  const response = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers,
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }
  return response.json();
}

export interface FsEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  isFile: boolean;
  isSymlink: boolean;
}

export const fs = {
  readdir: (path: string) =>
    request<{ items: FsEntry[] }>(`/api/fs/readdir?path=${encodeURIComponent(path)}`),
  read: (path: string) =>
    request<{ content: string; name: string; size: number }>(`/api/fs/read?path=${encodeURIComponent(path)}`),
  write: (path: string, content: string) =>
    request<{ ok: boolean }>("/api/fs/write", {
      method: "POST",
      body: JSON.stringify({ path, content }),
    }),
  create: (path: string, type: "file" | "directory", content?: string) =>
    request<{ ok: boolean }>("/api/fs/create", {
      method: "POST",
      body: JSON.stringify({ path, type, content }),
    }),
  rename: (from: string, to: string) =>
    request<{ ok: boolean }>("/api/fs/rename", {
      method: "POST",
      body: JSON.stringify({ from, to }),
    }),
  delete: (path: string) =>
    request<{ ok: boolean }>("/api/fs/delete", {
      method: "POST",
      body: JSON.stringify({ path }),
    }),
  stat: (path: string) =>
    request<{ path: string; name: string; size: number; isDirectory: boolean; modified: string }>(
      `/api/fs/stat?path=${encodeURIComponent(path)}`
    ),
};

export const terminal = {
  create: (options?: { cols?: number; rows?: number; cwd?: string }) =>
    request<{ id: string; cols: number; rows: number }>("/api/terminal/create", {
      method: "POST",
      body: JSON.stringify(options || {}),
    }),
  list: () =>
    request<{ sessions: Array<{ id: string; cols: number; rows: number }> }>("/api/terminal/list"),
  kill: (id: string) =>
    request<{ ok: boolean }>(`/api/terminal/${id}`, { method: "DELETE" }),
  connect: (id: string): WebSocket => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    const tokenParam = authToken ? `?token=${authToken}` : "";
    return new WebSocket(`${protocol}//${host}/api/terminal/${id}/ws${tokenParam}`);
  },
};

export interface CommandOutput {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  timedOut: boolean;
  truncated: boolean;
}

export const shell = {
  run: (command: string, cwd?: string, timeout?: number) =>
    request<CommandOutput>("/api/shell/run", {
      method: "POST",
      body: JSON.stringify({ command, cwd, timeout }),
    }),
};

export const ai = {
  chat: (messages: Array<{ role: string; content: string }>, sessionId: string) => {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (authToken) headers["Authorization"] = `Bearer ${authToken}`;
    return fetch(`${BASE_URL}/api/ai/chat`, {
      method: "POST",
      headers,
      body: JSON.stringify({ messages, sessionId }),
    });
  },
  stop: (sessionId: string) =>
    request<{ ok: boolean; stopped: boolean }>("/api/ai/stop", {
      method: "POST",
      body: JSON.stringify({ sessionId }),
    }),
};
