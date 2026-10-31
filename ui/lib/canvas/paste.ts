// What a paste onto the canvas adds. Plain code, so it runs in tests.

const WEB_URL = /^https?:\/\/\S+$/i;

/** Text at most this long, in characters and lines, becomes a sticky note; longer, a note. */
const STICKY_CHARS = 280;
const STICKY_LINES = 6;
/** A note made from pasted text is titled with its first line, cut to this many characters. */
const TITLE_CHARS = 80;

/** What pasted text adds to the canvas. */
export type Pasted =
  | { kind: "link"; url: string }
  | { kind: "files"; paths: string[] }
  | { kind: "sticky"; text: string }
  | { kind: "note"; title: string; body: string };

/** What the pasted `text` adds, or null when there is nothing to add. */
export function pasted(text: string): Pasted | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const url = pastedUrl(trimmed);
  if (url) return { kind: "link", url };
  const paths = pastedPaths(trimmed);
  if (paths) return { kind: "files", paths };
  return pastedText(trimmed);
}

/**
 * The web link the pasted text is, or null when it is anything else. Browsers copy addresses
 * with their scheme, and a bare `notes.md` is more likely a file name than a site.
 */
export function pastedUrl(text: string): string | null {
  const trimmed = text.trim();
  if (WEB_URL.test(trimmed)) return trimmed;
  return null;
}

/**
 * The files a file manager copied, one per line as `file://` addresses or absolute paths, or
 * null when any line is something else. Whether they exist is the host's to check.
 */
export function pastedPaths(text: string): string[] | null {
  const lines = text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const paths: string[] = [];
  for (const line of lines) {
    if (line.startsWith("file://")) {
      paths.push(decodeURIComponent(line.slice("file://".length)));
      continue;
    }
    if (!line.startsWith("/")) return null;
    paths.push(line);
  }
  if (paths.length === 0) return null;
  return paths;
}

/** Short text as a sticky note; longer as a note named after its first line, holding all of it. */
export function pastedText(text: string): Pasted {
  const lines = text.split("\n");
  if (text.length <= STICKY_CHARS && lines.length <= STICKY_LINES) return { kind: "sticky", text };
  const title = lines[0].replace(/^#+\s*/, "").trim().slice(0, TITLE_CHARS);
  return { kind: "note", title, body: text };
}

/** The name a pasted image is saved under, from when it was pasted: `Pasted image 2026-10-31 16-54.png`. */
export function pastedImageName(at: Date, mime: string): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
  const time = `${pad(at.getHours())}-${pad(at.getMinutes())}`;
  return `Pasted image ${day} ${time}.${extension(mime)}`;
}

function extension(mime: string): string {
  if (mime === "image/jpeg") return "jpg";
  if (mime === "image/svg+xml") return "svg";
  return mime.replace(/^image\//, "");
}
