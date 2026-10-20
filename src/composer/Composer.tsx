import { useLayoutEffect, useRef, useState } from 'react';
import { commands, type Workspace } from '../bindings';
import type { LiveSession } from '../sessions/registry';
import styles from './Composer.module.css';
import { toPastedPrompt } from './paste';

interface ComposerProps {
  workspace: Workspace;
  session: LiveSession;
  /** Close the composer and hand focus back to the terminal. */
  onClose: () => void;
}

const MAX_HEIGHT = 240;

/**
 * On-demand composer for long, dictated prompts (SPEC §6.3, amended 2026-10-20).
 * The terminal is the everyday input; ⌘L opens this when a big, calm field helps.
 * ↵ sends, ⇧↵ adds a line, esc closes. (Queueing arrives with the queue in M5.)
 * References: Ferndesk identity pill, Cursor controls row, Langdock multi-line field.
 */
export function Composer({ workspace, session, onClose }: ComposerProps) {
  const [text, setText] = useState('');
  const field = useRef<HTMLTextAreaElement>(null);
  const ended = session.exit !== undefined;

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
    onClose();
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
        autoFocus
        disabled={ended}
        placeholder={ended ? 'This session has ended.' : `Tell ${session.info.title} what to do…`}
        aria-label={`Message ${session.info.title} in ${workspace.name}`}
        onChange={(e) => {
          setText(e.target.value);
        }}
        onKeyDown={(e) => {
          // Don't send mid-composition (IME / dictation still finalising text).
          if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            send();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onClose();
          }
        }}
      />
      <div className={styles.controls}>
        <span className={styles.hint}>
          <kbd className={styles.kbd}>↵</kbd> send · <kbd className={styles.kbd}>⇧↵</kbd> new line ·{' '}
          <kbd className={styles.kbd}>esc</kbd> close
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
}
