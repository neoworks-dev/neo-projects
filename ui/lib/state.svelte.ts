// The window's state: where you are, what is selected, how the sync is doing. Views read the host
// again whenever `version` moves, which every "changed" event from the host does.
import { api, onChanged, onSync, type SyncStatus } from "./api.ts";

/** `inside` is the directory of the folder open within the project, workspace-relative. */
export type Route = { view: "overview" } | { view: "project"; project: string; inside?: string };

export type Selection =
  | { kind: "issue"; node: string }
  | { kind: "note"; path: string }
  | { kind: "node"; canvas: string; id: string };

class AppState {
  /** The host's commands, for components. */
  get api() {
    return api;
  }
  /** The workspace folder, or null before one is chosen. */
  workspace = $state<string | null>(null);
  /** False until the host said which workspace is open. */
  ready = $state(false);
  route = $state<Route>({ view: "overview" });
  selection = $state<Selection | null>(null);
  version = $state(0);
  sync = $state<SyncStatus>({
    running: false,
    pending: 0,
    failed: [],
    conflicts: 0,
    lastSync: null,
    error: null,
    login: null,
  });
  /** A short message at the bottom of the window, such as an error from the host. */
  toast = $state<string | null>(null);
  private toastTimer: ReturnType<typeof setTimeout> | undefined;

  /** Starts following the host; returns what stops it. */
  start(): () => void {
    const stops = [
      onChanged(() => this.version++),
      onSync((status) => {
        this.sync = status;
      }),
    ];
    void api.workspace().then(({ path }) => {
      this.workspace = path;
      this.ready = true;
    });
    void api.syncStatus().then((status) => (this.sync = status)).catch(() => {});
    return () => stops.forEach((stop) => stop());
  }

  async openWorkspace(path: string) {
    await this.run(async () => {
      const opened = await api.openWorkspace(path);
      this.workspace = opened.path;
      this.route = { view: "overview" };
      this.selection = null;
      this.version++;
    });
  }

  open(route: Route) {
    this.route = route;
    this.selection = null;
  }

  select(selection: Selection | null) {
    this.selection = selection;
  }

  /** Runs a host call, showing its error instead of throwing it. */
  async run<T>(call: () => Promise<T>): Promise<T | undefined> {
    try {
      return await call();
    } catch (error) {
      this.say(error instanceof Error ? error.message : String(error));
      return undefined;
    }
  }

  say(message: string) {
    this.toast = message;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = null), 5000);
  }
}

export const app = new AppState();
