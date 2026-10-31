// Every icon the window draws, by what it means. They come from Phosphor, the set the NeoWorks
// design system and neoworks.dev use: regular weight for chrome, fill only where a state is
// shown. Swapping the set, or one icon, happens here and nowhere else.
import arrowCounterClockwise from "@phosphor-icons/core/regular/arrow-counter-clockwise.svg";
import arrowSquareOut from "@phosphor-icons/core/regular/arrow-square-out.svg";
import arrowsInSimple from "@phosphor-icons/core/regular/arrows-in-simple.svg";
import arrowsOutSimple from "@phosphor-icons/core/regular/arrows-out-simple.svg";
import calendarBlank from "@phosphor-icons/core/regular/calendar-blank.svg";
import caretRight from "@phosphor-icons/core/regular/caret-right.svg";
import cellSignalFull from "@phosphor-icons/core/regular/cell-signal-full.svg";
import checkBold from "@phosphor-icons/core/bold/check-bold.svg";
import checkCircle from "@phosphor-icons/core/regular/check-circle.svg";
import checkCircleFill from "@phosphor-icons/core/fill/check-circle-fill.svg";
import checkSquareFill from "@phosphor-icons/core/fill/check-square-fill.svg";
import circle from "@phosphor-icons/core/regular/circle.svg";
import circleDashed from "@phosphor-icons/core/regular/circle-dashed.svg";
import circleHalf from "@phosphor-icons/core/regular/circle-half.svg";
import circleHalfFill from "@phosphor-icons/core/fill/circle-half-fill.svg";
import columns from "@phosphor-icons/core/regular/columns.svg";
import cornersOut from "@phosphor-icons/core/regular/corners-out.svg";
import fileText from "@phosphor-icons/core/regular/file-text.svg";
import folder from "@phosphor-icons/core/regular/folder.svg";
import folderSimple from "@phosphor-icons/core/regular/folder-simple.svg";
import githubLogo from "@phosphor-icons/core/regular/github-logo.svg";
import gridFour from "@phosphor-icons/core/regular/grid-four.svg";
import hash from "@phosphor-icons/core/regular/hash.svg";
import kanban from "@phosphor-icons/core/regular/kanban.svg";
import lightning from "@phosphor-icons/core/regular/lightning.svg";
import link from "@phosphor-icons/core/regular/link.svg";
import minus from "@phosphor-icons/core/regular/minus.svg";
import noteBlank from "@phosphor-icons/core/regular/note-blank.svg";
import notePencil from "@phosphor-icons/core/regular/note-pencil.svg";
import paperPlaneRight from "@phosphor-icons/core/regular/paper-plane-right.svg";
import playFill from "@phosphor-icons/core/fill/play-fill.svg";
import plus from "@phosphor-icons/core/regular/plus.svg";
import prohibit from "@phosphor-icons/core/regular/prohibit.svg";
import prohibitInset from "@phosphor-icons/core/fill/prohibit-inset-fill.svg";
import rows from "@phosphor-icons/core/regular/rows.svg";
import square from "@phosphor-icons/core/regular/square.svg";
import squaresFour from "@phosphor-icons/core/regular/squares-four.svg";
import tag from "@phosphor-icons/core/regular/tag.svg";
import textT from "@phosphor-icons/core/regular/text-t.svg";
import treeView from "@phosphor-icons/core/regular/tree-view.svg";
import users from "@phosphor-icons/core/regular/users.svg";
import x from "@phosphor-icons/core/regular/x.svg";
import xCircle from "@phosphor-icons/core/regular/x-circle.svg";
import xCircleFill from "@phosphor-icons/core/fill/x-circle-fill.svg";
import { tones } from "./theme.ts";

export const icons = {
  // Chrome
  mark: prohibit,
  close: x,
  add: plus,
  remove: minus,
  check: checkBold,
  next: caretRight,
  external: arrowSquareOut,
  expand: arrowsOutSimple,
  collapse: arrowsInSimple,
  fit: cornersOut,
  send: paperPlaneRight,
  reopen: arrowCounterClockwise,
  // Places
  overview: squaresFour,
  files: treeView,
  github: githubLogo,
  local: folderSimple,
  // Things on the canvas
  note: notePencil,
  sticky: noteBlank,
  link,
  board: kanban,
  // Arranging selected nodes: side by side is columns of one node, a stack is rows of one.
  arrangeGrid: gridFour,
  arrangeRow: columns,
  arrangeColumn: rows,
  // Properties
  status: circleDashed,
  field: circleHalf,
  priority: cellSignalFull,
  effort: lightning,
  date: calendarBlank,
  number: hash,
  text: textT,
  labels: tag,
  assignees: users,
  // Issue states, as a card's status icon
  stateBacklog: circleDashed,
  stateTodo: circle,
  stateInProgress: circleHalfFill,
  stateDone: checkCircleFill,
  stateNotPlanned: xCircleFill,
  closeDone: checkCircle,
  closeNotPlanned: xCircle,
  blocked: prohibitInset,
  // Task lists
  taskDone: checkSquareFill,
  taskOpen: square,
  // Files. Files and folders in a listing take their icons from `fileIcons.ts`.
  folder,
  folderClosed: folderSimple,
  fileText,
  /** On a video's poster. */
  play: playFill,
};

/** The icon and colour for an issue's state: by its Status when open, by why when closed. */
export function stateIcon(card: {
  state: string;
  stateReason: string | null;
  fields: Record<string, unknown>;
}): { icon: string; color: string } {
  if (card.state === "CLOSED") {
    if (card.stateReason === "NOT_PLANNED") return { icon: icons.stateNotPlanned, color: tones.GRAY };
    return { icon: icons.stateDone, color: tones.PURPLE };
  }
  const status = String(card.fields.Status ?? "").toLowerCase();
  if (status === "backlog") return { icon: icons.stateBacklog, color: tones.GRAY };
  if (status === "in progress" || status === "doing") return { icon: icons.stateInProgress, color: tones.YELLOW };
  if (status === "todo" || status === "to do") return { icon: icons.stateTodo, color: tones.BLUE };
  return { icon: icons.stateTodo, color: tones.GREEN };
}
