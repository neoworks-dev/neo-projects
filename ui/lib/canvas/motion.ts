// Moving the view smoothly: Fit and the zoom buttons ease to where they go instead of jumping.
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

/** The view zoomed to `zoom` about the centre of a viewport of `size`. */
export function zoomedAbout(view: ViewState, zoom: number, size: { width: number; height: number }): ViewState {
  const next = Math.min(2.5, Math.max(0.15, zoom));
  const centre = { x: view.x + size.width / 2 / view.zoom, y: view.y + size.height / 2 / view.zoom };
  return { x: centre.x - size.width / 2 / next, y: centre.y - size.height / 2 / next, zoom: next };
}
