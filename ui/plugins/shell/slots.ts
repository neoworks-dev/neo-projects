// The slots the shell declares: where the window's panes go.
import type { Chain, List } from "../../kernel/slots/types.ts";

declare module "../../kernel/slots/types.ts" {
  interface SlotMap {
    /** The top bar's start: where you are. */
    "shell.top": List;
    /** The top bar's end: sync state, buttons. */
    "shell.top.right": List;
    /** The tool rail at the left: where to go. */
    "shell.rail": List;
    /** The status bar's start: where you are. */
    "shell.status": List;
    /** The status bar's end: sync state, zoom. */
    "shell.status.right": List;
    /** What fills the window: the first entry whose `select` says yes (by route). */
    "shell.main": Chain;
    /** The panel at the right: the first entry whose `select` says yes (by selection). */
    "shell.panel": Chain;
    /** Drawn over everything: dialogs. */
    "shell.overlay": List;
  }
}
