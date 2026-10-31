// File and folder icons from the Material Icon Theme, the set VS Code users know. The build
// copies its SVGs to `dist/file-icons/`; this picks one by name, the way VS Code does. Plain code,
// so it runs in tests. Only the manifest's maps are imported, so the rest stays out of the bundle.
import {
  file,
  fileExtensions,
  fileNames,
  folder,
  folderExpanded,
  folderNames,
  folderNamesExpanded,
} from "material-icon-theme/dist/material-icons.json";

/** Extensions of this app's own files, which the theme doesn't know. */
const ours: Record<string, string> = {
  canvas: "excalidraw",
  board: "table",
};

/** The icon for a file or folder named `name`, as a path relative to the bundle. */
export function fileIcon(name: string, dir = false, open = false): string {
  return `file-icons/${iconName(name, dir, open)}.svg`;
}

function iconName(name: string, dir: boolean, open: boolean): string {
  const lower = name.toLowerCase();
  if (dir) {
    const names: Record<string, string> = open ? folderNamesExpanded : folderNames;
    return names[name] ?? names[lower] ?? (open ? folderExpanded : folder);
  }
  const named = (fileNames as Record<string, string>)[name] ?? (fileNames as Record<string, string>)[lower];
  if (named) return named;
  // The longest extension first: `x.test.ts` is a test before it is TypeScript.
  const parts = lower.split(".");
  for (let start = 1; start < parts.length; start++) {
    const extension = parts.slice(start).join(".");
    const icon = (fileExtensions as Record<string, string>)[extension] ?? ours[extension];
    if (icon) return icon;
  }
  return file;
}
