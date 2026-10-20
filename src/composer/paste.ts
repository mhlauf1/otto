/** Wrap text in bracketed-paste markers so a multi-line prompt arrives as one paste. */
export function toPastedPrompt(text: string): string {
  return `\x1b[200~${text.replace(/\r?\n/g, '\n')}\x1b[201~`;
}
