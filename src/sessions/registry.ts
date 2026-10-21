import { Channel } from '@tauri-apps/api/core';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal } from '@xterm/xterm';
import { create } from 'zustand';
import { commands, type AgentKind, type PtyEvent, type SessionInfo } from '../bindings';
import { font, terminal as palette } from '../design/tokens';

/**
 * Live terminal sessions. Each one owns an xterm `Terminal` that lives *outside* React,
 * so switching workspace or tab never loses scrollback. React only attaches it to the DOM.
 */
export interface LiveSession {
  info: SessionInfo;
  term: Terminal;
  fit: FitAddon;
  /** Undefined while running; the exit code (or null for a signal) once it ends. */
  exit: number | null | undefined;
}

interface SessionsState {
  sessions: LiveSession[];
  /** Selected session per workspace. A missing key means the workspace home view. */
  activeByWorkspace: Record<string, string | undefined>;
  open: (
    workspaceId: string,
    agent: AgentKind,
    options?: { signIn?: boolean; projectPath?: string | undefined; resumeId?: string | undefined },
  ) => Promise<LiveSession>;
  close: (id: string) => void;
  select: (workspaceId: string, id: string | undefined) => void;
  /** Called when a session ends (e.g. a sign-in flow finishes). */
  onExit?: (session: LiveSession) => void;
}

export class SessionError extends Error {}

function newTerminal(): { term: Terminal; fit: FitAddon } {
  const term = new Terminal({
    fontFamily: font.mono,
    fontSize: 13,
    lineHeight: 1.25,
    cursorBlink: true,
    allowProposedApi: false,
    macOptionIsMeta: true,
    scrollback: 10_000,
    theme: palette,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  return { term, fit };
}

export const useSessions = create<SessionsState>((set, get) => ({
  sessions: [],
  activeByWorkspace: {},

  open: async (workspaceId, agent, options = {}) => {
    const signIn = options.signIn ?? false;
    const { term, fit } = newTerminal();
    const channel = new Channel<PtyEvent>();
    let id = '';
    channel.onmessage = (event) => {
      if (event.kind === 'data') {
        term.write(event.text);
        return;
      }
      term.write(
        `\r\n\x1b[2m[process exited${event.code === null ? '' : ` with code ${String(event.code)}`}]\x1b[0m\r\n`,
      );
      set((s) => ({
        sessions: s.sessions.map((x) => (x.info.id === id ? { ...x, exit: event.code } : x)),
      }));
      const ended = get().sessions.find((x) => x.info.id === id);
      if (ended) get().onExit?.(ended);
    };

    const result = await commands.openSession(
      workspaceId,
      agent,
      signIn,
      options.projectPath ?? null,
      options.resumeId ?? null,
      100,
      30,
      channel,
    );
    if (result.status === 'error') {
      term.dispose();
      throw new SessionError(result.error.message);
    }
    id = result.data.id;

    // Every keystroke, paste and dictated phrase goes straight to the PTY.
    term.onData((data) => {
      void commands.writeSession(id, data);
    });
    term.onResize(({ cols, rows }) => {
      void commands.resizeSession(id, cols, rows);
    });

    const session: LiveSession = { info: result.data, term, fit, exit: undefined };
    set((s) => ({
      sessions: [...s.sessions, session],
      activeByWorkspace: { ...s.activeByWorkspace, [workspaceId]: id },
    }));
    return session;
  },

  close: (id) => {
    const session = get().sessions.find((s) => s.info.id === id);
    if (!session) return;
    void commands.closeSession(id);
    session.term.dispose();
    set((s) => {
      const remaining = s.sessions.filter((x) => x.info.id !== id);
      const ws = session.info.workspaceId;
      const next =
        s.activeByWorkspace[ws] === id
          ? remaining.filter((x) => x.info.workspaceId === ws).at(-1)?.info.id
          : s.activeByWorkspace[ws];
      return { sessions: remaining, activeByWorkspace: { ...s.activeByWorkspace, [ws]: next } };
    });
  },

  select: (workspaceId, id) => {
    set((s) => ({ activeByWorkspace: { ...s.activeByWorkspace, [workspaceId]: id } }));
  },
}));
