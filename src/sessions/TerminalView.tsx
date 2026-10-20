import { WebglAddon } from '@xterm/addon-webgl';
import '@xterm/xterm/css/xterm.css';
import { useEffect, useRef } from 'react';
import type { LiveSession } from './registry';
import styles from './TerminalView.module.css';

interface TerminalViewProps {
  session: LiveSession;
  visible: boolean;
}

/** Attaches a session's terminal to the DOM and keeps it sized to its container. */
export function TerminalView({ session, visible }: TerminalViewProps) {
  const host = useRef<HTMLDivElement>(null);

  // Open once. The Terminal outlives this component, but it's only ever opened here.
  useEffect(() => {
    const el = host.current;
    if (!el || session.term.element) return;
    session.term.open(el);
    try {
      // GPU rendering keeps input latency under budget (SPEC §5.4); the DOM renderer is the fallback.
      const webgl = new WebglAddon();
      webgl.onContextLoss(() => {
        webgl.dispose();
      });
      session.term.loadAddon(webgl);
    } catch {
      // WebGL unavailable: xterm keeps its DOM renderer.
    }
  }, [session]);

  // Fit to the container whenever it resizes or becomes visible.
  useEffect(() => {
    const el = host.current;
    if (!el || !visible) return;
    const fit = () => {
      if (el.clientWidth > 0 && el.clientHeight > 0) session.fit.fit();
    };
    fit();
    session.term.focus();
    const observer = new ResizeObserver(fit);
    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, [session, visible]);

  return (
    <div
      ref={host}
      className={styles.terminal}
      hidden={!visible}
      aria-label={`${session.info.title} terminal`}
      role="region"
    />
  );
}
