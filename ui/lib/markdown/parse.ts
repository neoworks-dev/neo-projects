// Markdown to tokens, and the questions the renderer asks of them. Plain code, so it runs in tests.
import { Lexer, type Token, type Tokens } from "marked";

/** GitHub-flavoured: tables, task lists, strikethrough and autolinks. */
export function parse(source: string): Token[] {
  return new Lexer({ gfm: true, breaks: false }).lex(source);
}

/** Every link in a run of inline tokens, in order. */
export function links(tokens: Token[]): string[] {
  const out: string[] = [];
  for (const token of tokens) {
    if (token.type === "link") out.push((token as Tokens.Link).href);
    if ("tokens" in token && token.tokens) out.push(...links(token.tokens));
  }
  return out;
}

/** The images of a paragraph made only of images (and space between them), else null. */
export function onlyImages(tokens: Token[]): Tokens.Image[] | null {
  const images = tokens.filter((t): t is Tokens.Image => t.type === "image");
  const rest = tokens.filter((t) => t.type !== "image" && !(t.type === "text" && !t.raw.trim()) && t.type !== "br");
  return images.length > 0 && rest.length === 0 ? images : null;
}

/**
 * Where an image's `src` points, for `Image`: web and data URLs as they are, absolute paths as
 * they are, anything else relative to `base` (the note's folder), with `%20` and friends decoded.
 */
export function resolveSrc(src: string, base: string | null): string {
  if (/^(https?|data):/i.test(src) || src.startsWith("/")) return src;
  let path = src;
  try {
    path = decodeURIComponent(src);
  } catch {
    // Not percent-encoded after all.
  }
  if (!base) return path;
  const parts = `${base.replace(/\/$/, "")}/${path}`.split("/");
  const out: string[] = [];
  for (const part of parts) {
    if (part === "..") out.pop();
    else if (part !== ".") out.push(part);
  }
  return out.join("/");
}
