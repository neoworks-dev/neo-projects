// Nodes let go over a folder: they fly over its middle at the size they shrank to while held
// there, the folder opens, and they shrink and sink into it before it closes again.
import type { Point, Rect } from "./geometry.ts";

/** How big a node held over a folder is, and how long the whole drop takes. */
export const HELD_SCALE = 0.55;
export const DROP_MS = 620;

/** Where in the drop, 0 to 1, the flight ends and the fall begins. */
const FLY_END = 0.4;
const FALL_START = 0.5;
/** How far a node sinks as it falls in, in world units. */
const SINK = 24;

/** A node `scale` times its size, shrunk towards `origin`, a point relative to its top left. */
export function scaledBox(node: Rect, origin: Point, scale: number): Rect {
  return {
    x: node.x + origin.x * (1 - scale),
    y: node.y + origin.y * (1 - scale),
    w: node.w * scale,
    h: node.h * scale,
  };
}

/** A node of `w` × `h` drawn `scale` times its size about its middle. */
export interface Frame {
  x: number;
  y: number;
  scale: number;
  opacity: number;
}

/**
 * Where a node of `w` × `h` is at `progress` of the drop, having been let go as the box `held`
 * over `folder`: it flies until its middle is the folder's, then shrinks away and sinks in.
 */
export function dropFrame(held: Rect, w: number, h: number, folder: Rect, progress: number): Frame {
  const scale = held.w / w;
  const start = { x: held.x + held.w / 2, y: held.y + held.h / 2 };
  const end = { x: folder.x + folder.w / 2, y: folder.y + folder.h / 2 };
  const fly = easeOut(part(progress, 0, FLY_END));
  const fall = easeIn(part(progress, FALL_START, 1));
  const middle = {
    x: start.x + (end.x - start.x) * fly,
    y: start.y + (end.y - start.y) * fly + SINK * fall,
  };
  return { x: middle.x - w / 2, y: middle.y - h / 2, scale: scale * (1 - fall), opacity: 1 - fall };
}

/** How far open the folder is at `progress` of the drop, 0 to 1: open as they arrive, closed as they vanish. */
export function folderOpening(progress: number): number {
  if (progress < 0.85) return easeOut(part(progress, FLY_END - 0.15, FALL_START));
  return 1 - easeOut(part(progress, 0.85, 1));
}

/** How far `progress` is from `from` to `to`, kept within 0 to 1. */
function part(progress: number, from: number, to: number): number {
  return Math.min(1, Math.max(0, (progress - from) / (to - from)));
}

function easeOut(t: number): number {
  return 1 - (1 - t) ** 3;
}

function easeIn(t: number): number {
  return t * t;
}
