// A set of claims that something is the case, each a function so that it can read reactive state:
// the keyboard is taken while any pane says so, the right column is shown while any section has
// something. Reactive, so what reads `held` follows both the claims and the state they read.

export class Claims {
  private list = $state.raw<readonly (() => boolean)[]>([]);

  /** Whether any claim holds now. */
  get held(): boolean {
    return this.list.some((claim) => claim());
  }

  /** Adds a claim; returns the call that withdraws it. */
  add(claim: () => boolean): () => void {
    this.list = [...this.list, claim];
    return () => (this.list = this.list.filter((other) => other !== claim));
  }
}
