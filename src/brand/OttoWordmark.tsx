interface OttoWordmarkProps {
  height: number;
}

/** The palindrome wordmark. The t-tails turn toward each other and stop short — the same gap as the mark. */
export function OttoWordmark({ height }: OttoWordmarkProps) {
  return (
    <svg
      height={height}
      width={(height * 160) / 56}
      viewBox="0 8 160 56"
      fill="none"
      stroke="currentColor"
      strokeWidth={7.5}
      role="img"
      aria-label="Otto"
    >
      <circle cx="20" cy="44" r="16" />
      <path d="M62 14V50A10 10 0 0 0 72 60" />
      <path d="M98 14V50A10 10 0 0 1 88 60" />
      <path d="M50 28.25H110" />
      <circle cx="140" cy="44" r="16" />
    </svg>
  );
}
