// What a paste onto the canvas adds. Plain code, so it runs in tests.

const WEB_URL = /^https?:\/\/\S+$/i;

/**
 * The web link the pasted text is, or null when it is anything else. Browsers copy addresses
 * with their scheme, and a bare `notes.md` is more likely a file name than a site.
 */
export function pastedUrl(text: string): string | null {
  const trimmed = text.trim();
  if (WEB_URL.test(trimmed)) return trimmed;
  return null;
}
