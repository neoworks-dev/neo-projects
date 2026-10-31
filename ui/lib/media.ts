// What a PDF's or video's card says under its poster. Plain code, so it runs in tests.

/** A video's length as a player shows it: `0:07`, `12:34`, `1:02:03`. */
export function duration(seconds: number): string {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = String(total % 60).padStart(2, "0");
  if (hours > 0) return `${hours}:${String(minutes).padStart(2, "0")}:${rest}`;
  return `${minutes}:${rest}`;
}

/** A PDF's page count: `1 page`, `12 pages`. */
export function pages(count: number): string {
  if (count === 1) return "1 page";
  return `${count} pages`;
}
