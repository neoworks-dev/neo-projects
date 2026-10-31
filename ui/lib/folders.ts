/**
 * The folders open within a project, outermost first, each with its directory: inside
 * `Garden/Seeds/Tomatoes` of the project `Garden`, Seeds and then Tomatoes.
 */
export function folderSteps(project: string, inside: string | undefined): { name: string; path: string }[] {
  if (!inside) return [];
  const names = inside.slice(project.length + 1).split("/");
  return names.map((name, i) => ({ name, path: [project, ...names.slice(0, i + 1)].join("/") }));
}
