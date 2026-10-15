/*
 * Glyph geometry for the working drawing.
 *
 * Coordinates are font units with y up, exactly as the font file states them.
 * The drawing flips y once, at the point of rendering; nothing here knows
 * about screens. These functions are pure so the component that calls them
 * stays a view and the arithmetic can be read on its own.
 */

export type NodeKind = 'on' | 'off' | 'implied';

export interface GlyphNode {
  x: number;
  y: number;
  /** on-curve point, off-curve control, or a TrueType implied midpoint. */
  t: NodeKind;
  /** smooth: the handles either side are kept collinear. */
  s: boolean;
}

/** A path command naming node indices: ['M', 3], ['Q', 4, 5], ['Z']. */
export type Command = [string, ...number[]];

export interface Specimen {
  key: string;
  char: string;
  glyphName: string;
  unicode: string;
  upem: number;
  advance: number;
  curveType: 'cubic' | 'quadratic' | string;
  points: number;
  implied: number;
  bbox: [number, number, number, number];
  /** [name, y] in font units: cap height, x-height, baseline, descender. */
  guides: [string, number][];
  ascender: number;
  descender: number;
  nodes: GlyphNode[];
  commands: Command[];
  /** [onCurveIndex, controlIndex] pairs, drawn as handle lines. */
  handles: [number, number][];
  path: string;
  source: string;
}

export interface SpecimenSet {
  /** [x, y, width, height] of the drawing frame, y already flipped. */
  frame: [number, number, number, number];
  guideLeft: number;
  guideRight: number;
  labelSize: number;
  latin: Specimen;
  cjk: Specimen;
  [key: string]: Specimen | number | number[] | undefined;
}

export interface Relations {
  /** on-curve index -> the controls that travel with it. */
  attached: Map<number, number[]>;
  /** control index -> the on-curve points it belongs to. */
  owners: Map<number, number[]>;
  /** implied index -> the two controls whose midpoint it is. */
  implied: Map<number, [number, number]>;
}

export const STRIKES = [0, 12, 16, 24] as const;

export function specimenOf(set: SpecimenSet, key: string): Specimen {
  const found = set[key];
  if (!found || typeof found !== 'object' || Array.isArray(found)) return set.latin;
  return found as Specimen;
}

export function copyNodes(specimen: Specimen): GlyphNode[] {
  return specimen.nodes.map((node) => ({ x: node.x, y: node.y, t: node.t, s: node.s }));
}

export function relationsOf(specimen: Specimen): Relations {
  const attached = new Map<number, number[]>();
  const owners = new Map<number, number[]>();
  for (const [anchor, control] of specimen.handles) {
    attached.set(anchor, [...(attached.get(anchor) ?? []), control]);
    owners.set(control, [...(owners.get(control) ?? []), anchor]);
  }

  // A TrueType contour may leave the on-curve point between two controls out
  // of the font file entirely; the extractor puts it back as 'implied', and
  // it has to keep following its two neighbours when either of them moves.
  const implied = new Map<number, [number, number]>();
  specimen.commands.forEach((command, index) => {
    if (command[0] !== 'Q') return;
    const end = command[2];
    if (end === undefined || specimen.nodes[end]?.t !== 'implied') return;
    const next = specimen.commands[index + 1];
    if (next && next[0] === 'Q' && next[1] !== undefined && command[1] !== undefined) {
      implied.set(end, [command[1], next[1]]);
    }
  });
  return { attached, owners, implied };
}

export function settleImplied(nodes: GlyphNode[], links: Relations): void {
  for (const [index, [first, second]] of links.implied) {
    const a = nodes[first];
    const b = nodes[second];
    const node = nodes[index];
    if (!a || !b || !node) continue;
    node.x = (a.x + b.x) / 2;
    node.y = (a.y + b.y) / 2;
  }
}

/**
 * Move one point, then repair everything that point owes its neighbours: an
 * on-curve point carries its handles, and a handle off a smooth point swings
 * the opposite handle to keep the tangent straight.
 */
export function moveNode(
  nodes: GlyphNode[],
  links: Relations,
  index: number,
  x: number,
  y: number,
): void {
  const node = nodes[index];
  if (!node || node.t === 'implied') return;
  const dx = x - node.x;
  const dy = y - node.y;
  node.x = x;
  node.y = y;

  if (node.t === 'on') {
    for (const control of links.attached.get(index) ?? []) {
      const handle = nodes[control];
      if (!handle) continue;
      handle.x += dx;
      handle.y += dy;
    }
  } else {
    for (const owner of links.owners.get(index) ?? []) {
      const anchor = nodes[owner];
      if (!anchor?.s) continue;
      const siblings = (links.attached.get(owner) ?? []).filter((other) => other !== index);
      if (siblings.length !== 1) continue;
      const sibling = nodes[siblings[0] as number];
      if (!sibling) continue;
      const length = Math.hypot(sibling.x - anchor.x, sibling.y - anchor.y);
      const reach = Math.hypot(anchor.x - node.x, anchor.y - node.y);
      if (reach < 1e-6 || length < 1e-6) continue;
      sibling.x = anchor.x + ((anchor.x - node.x) / reach) * length;
      sibling.y = anchor.y + ((anchor.y - node.y) / reach) * length;
    }
  }
  settleImplied(nodes, links);
}

/** The `d` attribute for the outline as the points currently stand. */
export function pathOf(specimen: Specimen, nodes: GlyphNode[]): string {
  const out: string[] = [];
  for (const command of specimen.commands) {
    if (command[0] === 'Z') {
      out.push('Z');
      continue;
    }
    const points: string[] = [];
    for (let index = 1; index < command.length; index += 1) {
      const node = nodes[command[index] as number];
      if (!node) continue;
      points.push(`${Math.round(node.x)} ${Math.round(-node.y)}`);
    }
    out.push(command[0] + points.join(' '));
  }
  return out.join('');
}

const FLATTEN_STEPS = 12;

function bezier(points: [number, number][], t: number): [number, number] {
  let run = points;
  while (run.length > 1) {
    const next: [number, number][] = [];
    for (let i = 0; i < run.length - 1; i += 1) {
      const a = run[i] as [number, number];
      const b = run[i + 1] as [number, number];
      next.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
    run = next;
  }
  return run[0] as [number, number];
}

/** The outline flattened into closed polygons, in font units. */
export function polygonsOf(specimen: Specimen, nodes: GlyphNode[]): [number, number][][] {
  const at = (index: number): [number, number] => {
    const node = nodes[index];
    return node ? [node.x, node.y] : [0, 0];
  };
  const out: [number, number][][] = [];
  let current: [number, number][] = [];
  let here: [number, number] = [0, 0];
  for (const command of specimen.commands) {
    if (command[0] === 'M') {
      current = [at(command[1] as number)];
      here = at(command[1] as number);
    } else if (command[0] === 'L') {
      current.push(at(command[1] as number));
      here = at(command[1] as number);
    } else if (command[0] === 'Q' || command[0] === 'C') {
      const run: [number, number][] = [here];
      for (let index = 1; index < command.length; index += 1) run.push(at(command[index] as number));
      for (let step = 1; step <= FLATTEN_STEPS; step += 1) run.length && current.push(bezier(run, step / FLATTEN_STEPS));
      here = at(command[command.length - 1] as number);
    } else if (command[0] === 'Z') {
      if (current.length > 2) out.push(current);
      current = [];
    }
  }
  return out;
}

export interface Pixel {
  x: number;
  y: number;
  size: number;
}

/**
 * Rasterise whatever outline is on screen right now: nonzero winding, sampled
 * at each cell's centre, on a grid anchored to the origin and the baseline so
 * a cell falls where a cell would fall in a real strike. Dragging a point
 * changes the pixels, which is the point of the two living in one document.
 */
export function strikeOf(specimen: Specimen, nodes: GlyphNode[], ppem: number): Pixel[] {
  if (ppem <= 0) return [];
  const cell = specimen.upem / ppem;
  const edges: [number, number, number, number][] = [];
  for (const polygon of polygonsOf(specimen, nodes)) {
    for (let index = 0; index < polygon.length; index += 1) {
      const a = polygon[index] as [number, number];
      const b = polygon[(index + 1) % polygon.length] as [number, number];
      if (a[1] !== b[1]) edges.push([a[0], a[1], b[0], b[1]]);
    }
  }
  if (!edges.length) return [];

  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const [x0, y0, x1, y1] of edges) {
    minX = Math.min(minX, x0, x1);
    maxX = Math.max(maxX, x0, x1);
    minY = Math.min(minY, y0, y1);
    maxY = Math.max(maxY, y0, y1);
  }

  const pixels: Pixel[] = [];
  const firstColumn = Math.floor(minX / cell);
  const lastColumn = Math.ceil(maxX / cell);
  for (let row = Math.floor(minY / cell); row < Math.ceil(maxY / cell); row += 1) {
    const y = (row + 0.5) * cell;
    const crossings: [number, number][] = [];
    for (const [x0, y0, x1, y1] of edges) {
      if ((y0 <= y && y < y1) || (y1 <= y && y < y0)) {
        const t = (y - y0) / (y1 - y0);
        crossings.push([x0 + (x1 - x0) * t, y1 > y0 ? 1 : -1]);
      }
    }
    if (!crossings.length) continue;
    crossings.sort((a, b) => a[0] - b[0]);

    let winding = 0;
    let cursor = 0;
    for (let column = firstColumn; column < lastColumn; column += 1) {
      const x = (column + 0.5) * cell;
      while (cursor < crossings.length && (crossings[cursor] as [number, number])[0] <= x) {
        winding += (crossings[cursor] as [number, number])[1];
        cursor += 1;
      }
      if (winding === 0) continue;
      pixels.push({ x: column * cell, y: -(row + 1) * cell, size: cell });
    }
  }
  return pixels;
}

/** The indices a person can actually grab, in contour order. */
export function draggableIndices(nodes: GlyphNode[]): number[] {
  const out: number[] = [];
  nodes.forEach((node, index) => {
    if (node.t !== 'implied') out.push(index);
  });
  return out;
}
