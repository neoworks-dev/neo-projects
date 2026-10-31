// Motion the window animates itself, frame by frame: the view easing to where Fit and the zoom
// buttons take it, and anything else that moves on the canvas.
/** What the viewport shows, as `Viewport` takes it. */
interface ViewState {
  x: number;
  y: number;
  zoom: number;
}

let animation = 0;

/**
 * Eases from `from` to `to` over `duration` ms, handing each frame's view to `set`. A new call
 * stops the one before. Zoom moves in log space, so zooming in and out feel the same speed.
 */
export function glide(from: ViewState, to: ViewState, set: (view: ViewState) => void, duration = 260) {
  cancelAnimationFrame(animation);
  const started = performance.now();
  const step = () => {
    const t = Math.min(1, (performance.now() - started) / duration);
    const eased = 1 - (1 - t) ** 3;
    const zoom = Math.exp(Math.log(from.zoom) + (Math.log(to.zoom) - Math.log(from.zoom)) * eased);
    // The world point at the top left moves so the centre's path is straight.
    set({ x: from.x + (to.x - from.x) * eased, y: from.y + (to.y - from.y) * eased, zoom });
    if (t < 1) animation = requestAnimationFrame(step);
  };
  animation = requestAnimationFrame(step);
}

/**
 * A CSS `cubic-bezier(x1, y1, x2, y2)` as a function from time to progress, both 0 to 1: the
 * design system gives its easings this way.
 */
export function cubicBezier([x1, y1, x2, y2]: number[]): (t: number) => number {
  const curve = (a: number, b: number, s: number) => 3 * a * s * (1 - s) ** 2 + 3 * b * s ** 2 * (1 - s) + s ** 3;
  const slope = (a: number, b: number, s: number) => 3 * a * (1 - s) ** 2 + 6 * (b - a) * s * (1 - s) + 3 * (1 - b) * s ** 2;
  return (t) => {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    // The curve's parameter whose x is `t`, by Newton's method from `t` itself.
    let s = t;
    for (let i = 0; i < 8; i++) {
      const d = slope(x1, x2, s);
      if (Math.abs(d) < 1e-6) break;
      s = Math.min(1, Math.max(0, s - (curve(x1, x2, s) - t) / d));
    }
    return curve(y1, y2, s);
  };
}

/**
 * Hands `set` a value going evenly from `from` to `to` over `duration` ms, once a frame, then
 * calls `done`. Answers with what stops it before then.
 */
export function tween(from: number, to: number, duration: number, set: (value: number) => void, done?: () => void): () => void {
  let frame = 0;
  const started = performance.now();
  const step = () => {
    const t = duration > 0 ? Math.min(1, (performance.now() - started) / duration) : 1;
    set(from + (to - from) * t);
    if (t < 1) {
      frame = requestAnimationFrame(step);
      return;
    }
    done?.();
  };
  frame = requestAnimationFrame(step);
  return () => cancelAnimationFrame(frame);
}

/** The view zoomed to `zoom` about the centre of a viewport of `size`. */
export function zoomedAbout(view: ViewState, zoom: number, size: { width: number; height: number }): ViewState {
  const next = Math.min(2.5, Math.max(0.15, zoom));
  const centre = { x: view.x + size.width / 2 / view.zoom, y: view.y + size.height / 2 / view.zoom };
  return { x: centre.x - size.width / 2 / next, y: centre.y - size.height / 2 / next, zoom: next };
}
