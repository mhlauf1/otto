import styles from './OttoMark.module.css';

interface OttoMarkProps {
  /** Pixel size of the square mark. */
  size: number;
  /** Which half carries the active workspace accent. `none` renders the plain mark. */
  lit?: 'left' | 'right' | 'none';
  /** Accessible name. Omit when the mark is decorative. */
  label?: string;
}

/**
 * The Otto mark: one ring split by a hard line — two identities, never touching.
 * Left half = first workspace (Playbook), right half = second (Lauf).
 */
export function OttoMark({ size, lit = 'none', label }: OttoMarkProps) {
  const half = (side: 'left' | 'right') =>
    lit === 'none' ? styles.plain : lit === side ? styles.lit : styles.dim;

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      role={label ? 'img' : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      <path
        className={half('left')}
        d="M29 6.174A26 26 0 0 0 29 57.826L29 46.697A15 15 0 0 1 29 17.303Z"
      />
      <path
        className={half('right')}
        d="M35 6.174A26 26 0 0 1 35 57.826L35 46.697A15 15 0 0 0 35 17.303Z"
      />
    </svg>
  );
}
