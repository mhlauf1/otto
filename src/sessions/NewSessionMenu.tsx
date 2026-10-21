import * as Menu from '@radix-ui/react-dropdown-menu';
import type { AgentKind } from '../bindings';
import styles from './NewSessionMenu.module.css';

const ITEMS: { agent: AgentKind; label: string; detail: string; shortcut?: string }[] = [
  { agent: 'claude', label: 'Claude Code', detail: 'Anthropic agent', shortcut: '⌘T' },
  { agent: 'codex', label: 'Codex', detail: 'OpenAI agent' },
  { agent: 'shell', label: 'Shell', detail: 'Terminal signed in as this workspace' },
];

interface NewSessionMenuProps {
  /** Where the session opens, e.g. "Lauf · stoc" or "Lauf · General". */
  target: string;
  onPick: (agent: AgentKind) => void;
  /** `row` is the full-width "+ New session"; `icon` is the "+" on a project row. */
  variant?: 'row' | 'icon';
}

/** One "+ New" entry point; Claude first, Shell available but secondary. */
export function NewSessionMenu({ target, onPick, variant = 'row' }: NewSessionMenuProps) {
  return (
    <Menu.Root>
      <Menu.Trigger
        className={variant === 'icon' ? styles.iconTrigger : styles.trigger}
        aria-label={`New session in ${target}`}
        title={variant === 'icon' ? `New session in ${target}` : undefined}
      >
        {variant === 'icon' ? '+' : '+ New session'}
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Content className={styles.menu} side="bottom" align="start" sideOffset={6}>
          <Menu.Label className={styles.label}>New in {target}</Menu.Label>
          {ITEMS.map((item) => (
            <Menu.Item
              key={item.agent}
              className={styles.item}
              onSelect={() => {
                onPick(item.agent);
              }}
            >
              <span className={styles.text}>
                <span>{item.label}</span>
                <span className={styles.detail}>{item.detail}</span>
              </span>
              {item.shortcut ? <kbd className={styles.kbd}>{item.shortcut}</kbd> : null}
            </Menu.Item>
          ))}
        </Menu.Content>
      </Menu.Portal>
    </Menu.Root>
  );
}
