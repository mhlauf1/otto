import { forwardRef, useImperativeHandle, useLayoutEffect, useRef, useState } from 'react';
import { commands, type Workspace } from '../bindings';
import type { LiveSession } from '../sessions/registry';
import styles from './Composer.module.css';
import { toPastedPrompt } from './paste';

export interface ComposerHandle {
  focus: () => void;
}

interface ComposerProps {
  workspace: Workspace;
  session: LiveSession;
}

const MAX_HEIGHT = 240;

/**
 * The dictation-friendly composer (SPEC §6.3): a large, calm field that Wispr Flow can
 * type into. ⌘↵ sends to the active session. (⌥↵ queueing arrives with the queue in M5.)
 * References: Ferndesk identity pill, Cursor controls row, Langdock multi-line field.
 */
export const Composer = forwardRef<ComposerHandle, ComposerProps>(function Composer(
  { workspace, session },
  ref,
) {
  const [text, setText] = useState('');
  const field = useRef<HTMLTextAreaElement>(null);
  const ended = session.exit !== undefined;

  useImperativeHandle(ref, () => ({ focus: () => field.current?.focus() }), []);

  // Grow with the text up to a cap, then scroll.
  useLayoutEffect(() => {
    const el = field.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${String(Math.min(el.scrollHeight, MAX_HEIGHT))}px`;
  }, [text]);

  const send = () => {
    const prompt = text.trim();
    if (!prompt || ended) return;
    const id = session.info.id;
    void commands.writeSession(id, toPastedPrompt(prompt)).then(() => {
      // Give the TUI a beat to finish the paste before pressing Return.
      window.setTimeout(() => {
        void commands.writeSession(id, '\r');
      }, 40);
    });
    setText('');
    session.term.scrollToBottom();
  };

  return (
    <form
      className={styles.composer}
      onSubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <div className={styles.header}>
        <span className={styles.identity}>
          <span className={styles.dot} aria-hidden />
          {workspace.name} · {workspace.identity.githubUser}
        </span>
        <span className={styles.target}>to {session.info.title}</span>
      </div>
      <textarea
        ref={field}
        className={styles.field}
        rows={2}
        value={text}
        disabled={ended}
        placeholder={ended ? 'This session has ended.' : `Tell ${session.info.title} what to do…`}
        aria-label={`Message ${session.info.title} in ${workspace.name}`}
        onChange={(e) => {
          setText(e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && e.metaKey) {
            e.preventDefault();
            send();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            session.term.focus();
          }
        }}
      />
      <div className={styles.controls}>
        <span className={styles.hint}>
          <kbd className={styles.kbd}>⌘↵</kbd> send · <kbd className={styles.kbd}>esc</kbd> back to
          terminal · <kbd className={styles.kbd}>⌘L</kbd> focus
        </span>
        <button
          type="submit"
          className={styles.send}
          disabled={ended || !text.trim()}
          aria-label="Send"
        >
          ↑
        </button>
      </div>
    </form>
  );
});
