// What the project view's keys act on; the view sets it while it is shown.
class ProjectKeys {
  /** Removes the selected node from the canvas or the selected relationship; false if none. */
  remove: (() => boolean) | null = null;
  /** Adds a note in the middle of the view and opens it. */
  note: (() => void) | null = null;
  /** Adds the link on the clipboard to the canvas. */
  paste: (() => void) | null = null;
  /** Adds a sticky note in the middle of the view, ready to write in. */
  sticky: (() => void) | null = null;
  /** Shows or hides the file explorer. */
  files: (() => void) | null = null;
  /** Turns the pen on, or off when it is on. */
  pen: (() => void) | null = null;
  /** Turns the eraser on, or off when it is on. */
  eraser: (() => void) | null = null;
  /** Turns the pen or eraser off; false when neither was on. */
  stopDrawing: (() => boolean) | null = null;
}

export const projectKeys = new ProjectKeys();
