/**
 * The tool strip (design spec §5).
 *
 * Every tool in the table is registered so the strip, the tooltips and the
 * keyboard map are complete and stable. All of them are live except Text,
 * which registers its key and its cursor and stays inert until the Text tab
 * arrives.
 *
 * Tap/hold: a tap switches tool, holding the key for 200 ms borrows the tool
 * until the key is released, which the strip shows with a dotted outline.
 */

import {
  Circle,
  Hand,
  MousePointer2,
  Pencil,
  PenTool,
  Ruler,
  RotateCw,
  Scaling,
  Scissors,
  Square,
  Type,
  ZoomIn,
  type LucideIcon,
} from 'lucide-react'

/** Identifier of one tool. */
export type ToolId =
  | 'select'
  | 'pen'
  | 'pencil'
  | 'knife'
  | 'rectangle'
  | 'ellipse'
  | 'measure'
  | 'rotate'
  | 'scale'
  | 'zoom'
  | 'hand'
  | 'text'

/** One row of the tool strip. */
export interface ToolDefinition {
  id: ToolId
  /** Single-key shortcut, lower case. */
  key: string
  /** Whether the shortcut needs Shift (⇧F for the ellipse). */
  shift: boolean
  /** i18n key of the tool name. */
  labelKey: string
  icon: LucideIcon
  /** CSS cursor while the tool is active. */
  cursor: string
  /** Whether the tool does anything yet; only Text is still inert. */
  live: boolean
  /** Visual grouping in the strip, separated by a hairline. */
  group: 'select' | 'draw' | 'shape' | 'transform' | 'view'
}

/** How long a tool key must be held before it is borrowed, in ms. */
export const HOLD_BORROW_MS = 200

/** The tool table, in strip order (design spec §5). */
export const TOOLS: readonly ToolDefinition[] = [
  {
    id: 'select',
    key: 'v',
    shift: false,
    labelKey: 'tool.select',
    icon: MousePointer2,
    cursor: 'default',
    live: true,
    group: 'select',
  },
  {
    id: 'pen',
    key: 'p',
    shift: false,
    labelKey: 'tool.pen',
    icon: PenTool,
    cursor: 'crosshair',
    live: true,
    group: 'draw',
  },
  {
    id: 'pencil',
    key: 'b',
    shift: false,
    labelKey: 'tool.pencil',
    icon: Pencil,
    cursor: 'crosshair',
    live: true,
    group: 'draw',
  },
  {
    id: 'knife',
    key: 'e',
    shift: false,
    labelKey: 'tool.knife',
    icon: Scissors,
    cursor: 'crosshair',
    live: true,
    group: 'draw',
  },
  {
    id: 'rectangle',
    key: 'f',
    shift: false,
    labelKey: 'tool.rectangle',
    icon: Square,
    cursor: 'crosshair',
    live: true,
    group: 'shape',
  },
  {
    id: 'ellipse',
    key: 'f',
    shift: true,
    labelKey: 'tool.ellipse',
    icon: Circle,
    cursor: 'crosshair',
    live: true,
    group: 'shape',
  },
  {
    id: 'measure',
    key: 'l',
    shift: false,
    labelKey: 'tool.measure',
    icon: Ruler,
    cursor: 'crosshair',
    live: true,
    group: 'transform',
  },
  {
    id: 'rotate',
    key: 'r',
    shift: false,
    labelKey: 'tool.rotate',
    icon: RotateCw,
    cursor: 'crosshair',
    live: true,
    group: 'transform',
  },
  {
    id: 'scale',
    key: 's',
    shift: false,
    labelKey: 'tool.scale',
    icon: Scaling,
    cursor: 'crosshair',
    live: true,
    group: 'transform',
  },
  {
    id: 'zoom',
    key: 'z',
    shift: false,
    labelKey: 'tool.zoom',
    icon: ZoomIn,
    cursor: 'zoom-in',
    live: true,
    group: 'view',
  },
  {
    id: 'hand',
    key: 'h',
    shift: false,
    labelKey: 'tool.hand',
    icon: Hand,
    cursor: 'grab',
    live: true,
    group: 'view',
  },
  {
    id: 'text',
    key: 't',
    shift: false,
    labelKey: 'tool.text',
    icon: Type,
    cursor: 'text',
    live: false,
    group: 'view',
  },
]

/** The tool a definition table lookup by id, always defined for a `ToolId`. */
export function toolById(id: ToolId): ToolDefinition {
  return TOOLS.find((tool) => tool.id === id) ?? (TOOLS[0] as ToolDefinition)
}

/** The tool a keystroke selects, or `null` when the key is not a tool key. */
export function toolForKey(key: string, shift: boolean): ToolId | null {
  const lower = key.toLowerCase()
  const exact = TOOLS.find((tool) => tool.key === lower && tool.shift === shift)
  if (exact) return exact.id
  // ⇧F is the ellipse; every other tool ignores Shift so a shifted key still
  // reaches its tool rather than doing nothing.
  const relaxed = TOOLS.find((tool) => tool.key === lower && !tool.shift)
  return relaxed?.id ?? null
}
