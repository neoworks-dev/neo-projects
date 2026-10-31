// The relationships an arrow between nodes can stand for, kept free of Svelte so it is easy to test.

/** What decides the relationships a node can have: the issue it shows, if any. */
export interface Issueish {
  local?: boolean;
}

export interface RelationOption {
  type: string;
  label: string;
  /** Where the relationship is kept. */
  where: string;
}

/** The relationships from `from` to `to`: issues of the same kind have their tracker's own. */
function optionsFor(from: Issueish | null, to: Issueish | null): RelationOption[] {
  const relates = { type: "relates", label: "Relates to", where: "Kept in relations.jsonl" };
  if (from && to && Boolean(from.local) === Boolean(to.local)) {
    let where = "On GitHub";
    if (from.local) where = "Kept in the issue files";
    return [
      { type: "blocks", label: "Blocks", where },
      { type: "parent-of", label: "Parent of, as a sub-issue", where },
      relates,
    ];
  }
  return [
    { type: "blocks", label: "Blocks", where: "Kept in relations.jsonl" },
    { type: "derived-from", label: "Derived from", where: "Kept in relations.jsonl" },
    relates,
  ];
}

/**
 * The relationships every one of `from` can have to `to`, for one arrow drawn from all of them.
 * An issue has one parent, so several can't all be its parent.
 */
export function relationOptions(from: (Issueish | null)[], to: Issueish | null): RelationOption[] {
  if (from.length === 0) return [];
  const each = from.map((one) => optionsFor(one, to));
  const shared = each[0].filter((option) => each.every((options) => options.some((o) => o.type === option.type)));
  return shared.flatMap((option) => {
    if (option.type === "parent-of" && from.length > 1) return [];
    const wheres = new Set(each.map((options) => options.find((o) => o.type === option.type)!.where));
    if (wheres.size > 1) return [{ ...option, where: "Kept where each pair keeps it" }];
    return [option];
  });
}
