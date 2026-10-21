import * as Dialog from '@radix-ui/react-dialog';
import { useState } from 'react';
import styles from './Sheet.module.css';

interface NameDialogProps {
  title: string;
  description?: string;
  label: string;
  initial?: string;
  submitLabel: string;
  /** Resolves on success; rejects with a message to show under the field. */
  onSubmit: (name: string) => Promise<void>;
  onClose: () => void;
}

/** A small sheet with one text field, for naming things like project groups. */
export function NameDialog({
  title,
  description,
  label,
  initial = '',
  submitLabel,
  onSubmit,
  onClose,
}: NameDialogProps) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSubmit(name);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'That didn’t work. Try again.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.overlay} />
        <Dialog.Content className={[styles.sheet, styles.compact].join(' ')}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
          >
            <header className={styles.header}>
              <Dialog.Title className={styles.title}>{title}</Dialog.Title>
              {description ? (
                <Dialog.Description className={styles.description}>
                  {description}
                </Dialog.Description>
              ) : (
                <Dialog.Description className={styles.visuallyHidden}>{label}</Dialog.Description>
              )}
            </header>
            <div className={styles.body}>
              <input
                className={styles.search}
                aria-label={label}
                placeholder={label}
                value={name}
                autoFocus
                maxLength={40}
                onChange={(e) => {
                  setName(e.target.value);
                }}
              />
              {error ? (
                <p className={styles.reason} role="alert">
                  {error}
                </p>
              ) : null}
            </div>
            <footer className={styles.footer}>
              <Dialog.Close className={styles.secondary} type="button">
                Cancel
              </Dialog.Close>
              <button
                className={styles.primary}
                type="submit"
                disabled={saving || name.trim() === ''}
              >
                {submitLabel}
              </button>
            </footer>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
