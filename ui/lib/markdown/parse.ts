// Markdown to tokens, and the questions the renderer asks of them. Plain code, so it runs in tests.
import { Marked, type TokenizerExtension, type Token, type Tokens } from "marked";

const embed = /^!\[\[([^\]|]+)(?:\|([^\]]*))?\]\]/;

/** Obsidian's `![[photo.png]]` and `![[photo.png|caption]]`, read as an ordinary image. */
const obsidianEmbed: TokenizerExtension = {
  name: "obsidianEmbed",
  level: "inline",
  start: (src) => {
    const at = src.indexOf("![[");
    if (at < 0) return undefined;
    return at;
  },
  tokenizer(src) {
    const match = embed.exec(src);
    if (!match) return undefined;
    const href = match[1].trim();
    const text = match[2]?.trim() ?? "";
    const image: Tokens.Image = { type: "image", raw: match[0], href, title: null, text, tokens: [] };
    return image;
  },
};

/** A LaTeX formula, `$…$` in a line or `$$…$$` on lines of its own (display style). */
export interface MathToken {
  type: "math";
  raw: string;
  text: string;
  display: boolean;
}

const displayBlock = /^\$\$[ \t]*\n?([\s\S]+?)\n?[ \t]*\$\$[ \t]*(?:\n+|$)/;
const displayInline = /^\$\$([\s\S]+?)\$\$/;
// As GitHub reads it: no space just inside the dollars, and no digit right after the closing
// one, so "$5 and $10" stays text.
const inlineMath = /^\$(?![\s$])((?:\\[\s\S]|[^\\$])+?)(?<!\s)\$(?!\d)/;

function startOfMath(src: string): number | undefined {
  const at = src.indexOf("$");
  if (at < 0) return undefined;
  return at;
}

const mathBlock: TokenizerExtension = {
  name: "math",
  level: "block",
  // Where a paragraph may end for a block: `$$` at the start of a line only.
  start: (src) => {
    const match = /(^|\n)\$\$/.exec(src);
    if (!match) return undefined;
    return match.index + match[1].length;
  },
  tokenizer(src) {
    const match = displayBlock.exec(src);
    if (!match) return undefined;
    const token: MathToken = { type: "math", raw: match[0], text: match[1].trim(), display: true };
    return token;
  },
};

const mathInline: TokenizerExtension = {
  name: "math",
  level: "inline",
  start: startOfMath,
  tokenizer(src) {
    const display = displayInline.exec(src);
    if (display) {
      const token: MathToken = { type: "math", raw: display[0], text: display[1].trim(), display: true };
      return token;
    }
    const match = inlineMath.exec(src);
    if (!match) return undefined;
    const token: MathToken = { type: "math", raw: match[0], text: match[1], display: false };
    return token;
  },
};

/**
 * GitHub-flavoured: tables, task lists, strikethrough and autolinks. And what Obsidian and GitHub
 * add: image embeds and LaTeX math.
 */
const markdown = new Marked({ gfm: true, breaks: false, extensions: [obsidianEmbed, mathBlock, mathInline] });

export function parse(source: string): Token[] {
  return markdown.lexer(source);
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

/** The images among a run of inline tokens, nested ones included, in order. */
export function images(tokens: Token[]): Tokens.Image[] {
  const out: Tokens.Image[] = [];
  for (const token of tokens) {
    if (token.type === "image") out.push(token as Tokens.Image);
    else if ("tokens" in token && token.tokens) out.push(...images(token.tokens));
  }
  return out;
}

/**
 * The places an image's `src` may be, to try in order: a web or data URL or an absolute path is
 * the one place; a relative path is looked for in the note's folder, then the workspace.
 */
export function imageSources(src: string, folder: string | null, workspace: string | null): string[] {
  if (/^(https?|data):/i.test(src) || src.startsWith("/")) return [src];
  const out: string[] = [];
  for (const base of [folder, workspace]) {
    if (!base) continue;
    const path = resolveSrc(src, base);
    if (!out.includes(path)) out.push(path);
  }
  if (out.length === 0) out.push(resolveSrc(src, null));
  return out;
}

/** Markdown that shows the image `name` beside the note; spaces would end the link, so they're encoded. */
export function imageLink(name: string): string {
  return `![](${name.replaceAll(" ", "%20")})`;
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
