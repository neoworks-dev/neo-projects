// Markdown as plain text, for the few lines a card shows: the syntax goes, the words stay.

const fence = /^\s*(```|~~~)/;
const heading = /^\s{0,3}#{1,6}\s+/;
const listMarker = /^\s*(?:[-*+]|\d+[.)])\s+(?:\[[ xX]\]\s+)?/;
const quote = /^\s*>\s?/;
const rule = /^\s*([-*_])(\s*\1){2,}\s*$/;
const image = /!\[([^\]]*)\]\([^)]*\)/g;
const wikiEmbed = /!\[\[([^\]|]+)(?:\|[^\]]*)?\]\]/g;
const wikiLink = /\[\[([^\]|]+)(?:\|([^\]]*))?\]\]/g;
const link = /\[([^\]]*)\]\([^)]*\)/g;
const emphasis = /(\*\*|__|\*|_|~~)(?=\S)(.+?)(?<=\S)\1/g;
const inlineCode = /`([^`]*)`/g;
const html = /<[^>]+>/g;

/** The text of `markdown` without its syntax, lines joined by spaces, at most `limit` characters. */
export function plainText(markdown: string, limit = 280): string {
  const words: string[] = [];
  let inFence = false;
  for (const raw of markdown.split("\n")) {
    if (fence.test(raw)) {
      inFence = !inFence;
      continue;
    }
    // Tables read badly as a run of words, so a card leaves them out.
    if (inFence || rule.test(raw) || raw.trimStart().startsWith("|")) continue;
    const line = raw
      .replace(heading, "")
      .replace(quote, "")
      .replace(listMarker, "")
      .replace(wikiEmbed, "")
      .replace(image, "")
      .replace(wikiLink, (_, target: string, label?: string) => label || target)
      .replace(link, "$1")
      .replace(inlineCode, "$1")
      .replace(emphasis, "$2")
      .replace(html, "")
      .trim();
    if (line) words.push(line);
    if (words.join(" ").length > limit) break;
  }
  const text = words.join(" ");
  return text.length > limit ? `${text.slice(0, limit - 1).trimEnd()}…` : text;
}
