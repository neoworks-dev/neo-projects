// What the project view's keys act on; the view sets it while it is shown.
class ProjectKeys {
  /** Removes the selected node from the canvas or the selected relationship; false if none. */
  remove: (() => boolean) | null = null;
  /** Adds a note in the middle of the view and opens it. */
  note: (() => void) | null = null;
  /** Opens the field for adding a link. */
  link: (() => void) | null = null;
  /** Shows or hides the file explorer. */
  files: (() => void) | null = null;
}

export const projectKeys = new ProjectKeys();
