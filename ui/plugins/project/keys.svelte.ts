// What the project view's keys act on; the view sets it while it is shown.
class ProjectKeys {
  /** Removes the selected node from the canvas or the selected relationship; false if none. */
  remove: (() => boolean) | null = null;
}

export const projectKeys = new ProjectKeys();
