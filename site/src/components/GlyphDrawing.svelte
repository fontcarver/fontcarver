<script lang="ts">
  /*
   * A working drawing of one real glyph: the outline, its points, its metrics
   * as dimensions, and the strike rasterised from whatever is on screen.
   *
   * Astro renders this statically so the drawing is in the HTML, then
   * src/lib/enhance.ts mounts it again with live={true}. Controls exist only
   * in the live copy, so a page without JavaScript never shows a control that
   * does nothing. Geometry is set through SVG attributes, never style=, which
   * the site's content security policy forbids.
   */
  import {
    copyNodes,
    draggableIndices,
    moveNode,
    pathOf,
    relationsOf,
    settleImplied,
    specimenOf,
    strikeOf,
    STRIKES,
    type GlyphNode,
    type Specimen,
    type SpecimenSet,
  } from '../lib/glyph';

  interface Labels {
    specimen: string;
    reset: string;
    point: string;
    points: string;
    advance: string;
    curves: string;
    strike: string;
    off: string;
    none: string;
    corner: string;
    smooth: string;
    handle: string;
    implied: string;
    cubic: string;
    quadratic: string;
    hint: string;
    figure: string;
  }

  let {
    data,
    labels,
    live = false,
  }: { data: SpecimenSet; labels: Labels; live?: boolean } = $props();

  /** A fresh, editable copy of a specimen's points, with the implied ones placed. */
  function seed(of: Specimen): GlyphNode[] {
    const copy = copyNodes(of);
    settleImplied(copy, relationsOf(of));
    return copy;
  }

  let key = $state('latin');
  let strike = $state(0);
  let selected = $state(-1);
  // svelte-ignore state_referenced_locally
  // The specimen set is static data imported at build time; this is its
  // initial copy, and choose() makes every later one.
  let nodes = $state<GlyphNode[]>(seed(specimenOf(data, 'latin')));
  let svg: SVGSVGElement | undefined = $state();
  let frame: HTMLDivElement | undefined = $state();
  let plate: SVGGElement | undefined = $state();
  let dragging = -1;

  const specimen = $derived(specimenOf(data, key));
  const links = $derived(relationsOf(specimen));
  const outline = $derived(pathOf(specimen, nodes));
  const ppem = $derived(STRIKES[strike] ?? 0);
  const pixels = $derived(ppem > 0 ? strikeOf(specimen, nodes, ppem) : []);
  // The glyph rides centred on its advance, so switching specimen never moves
  // the drawing out from under the pointer.
  const shift = $derived(Math.round(data.frame[0] + data.frame[2] / 2 - specimen.advance / 2));
  // Where the advance dimension is drawn: near the bottom edge of the frame,
  // under the descender, the way a drawing carries its overall width.
  const baseline = $derived(data.frame[1] + data.frame[3] - 90);

  function describe(node: GlyphNode): string {
    if (node.t === 'off') return labels.handle;
    if (node.t === 'implied') return labels.implied;
    return node.s ? labels.smooth : labels.corner;
  }

  const readout = $derived.by(() => {
    const node = nodes[selected];
    if (!node) return labels.none;
    return `${Math.round(node.x)}, ${Math.round(node.y)} ${describe(node)}`;
  });

  function toFontUnits(event: PointerEvent): { x: number; y: number } | null {
    if (!svg || !plate) return null;
    const matrix = plate.getScreenCTM();
    if (!matrix) return null;
    const point = svg.createSVGPoint();
    point.x = event.clientX;
    point.y = event.clientY;
    const local = point.matrixTransform(matrix.inverse());
    return { x: local.x, y: -local.y };
  }

  function grab(event: PointerEvent): void {
    const target = event.target as SVGElement | null;
    const index = Number(target?.dataset?.index);
    if (!target?.dataset || Number.isNaN(index)) return;
    dragging = index;
    selected = index;
    svg?.setPointerCapture(event.pointerId);
    frame?.focus();
    event.preventDefault();
  }

  function drag(event: PointerEvent): void {
    if (dragging < 0) return;
    const point = toFontUnits(event);
    if (!point) return;
    moveNode(nodes, links, dragging, Math.round(point.x), Math.round(point.y));
    nodes = nodes;
  }

  function drop(event: PointerEvent): void {
    if (dragging < 0) return;
    dragging = -1;
    if (svg?.hasPointerCapture(event.pointerId)) svg.releasePointerCapture(event.pointerId);
  }

  function step(direction: number): void {
    const order = draggableIndices(nodes);
    if (!order.length) return;
    const at = order.indexOf(selected);
    selected = order[(at < 0 ? 0 : at + direction + order.length) % order.length] ?? -1;
  }

  const NUDGES: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, 1],
    ArrowDown: [0, -1],
  };

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'PageDown' || event.key === 'PageUp') {
      step(event.key === 'PageDown' ? 1 : -1);
      event.preventDefault();
      return;
    }
    const nudge = NUDGES[event.key];
    if (!nudge) return;
    if (selected < 0) step(1);
    const node = nodes[selected];
    if (!node) return;
    const amount = event.shiftKey ? 10 : 1;
    moveNode(nodes, links, selected, node.x + nudge[0] * amount, node.y + nudge[1] * amount);
    nodes = nodes;
    event.preventDefault();
  }

  function reset(): void {
    nodes = seed(specimen);
    selected = -1;
  }

  // The specimen switch is the only thing that changes which glyph is drawn,
  // so it makes the new copy of the points itself rather than through an
  // effect watching for it.
  function choose(next: string): void {
    key = next;
    strike = 0;
    selected = -1;
    nodes = seed(specimenOf(data, next));
  }
</script>

<figure class="plate" data-specimen={key} data-strike={ppem > 0 ? 'on' : 'off'}>
  <div class="plate__bar">
    {#if live}
      <div class="plate__tabs" role="tablist" aria-label={labels.specimen}>
        {#each ['latin', 'cjk'] as name (name)}
          {@const item = specimenOf(data, name)}
          <button
            class="plate__tab"
            type="button"
            role="tab"
            aria-selected={key === name}
            onclick={() => choose(name)}
          >
            <span class="plate__tabChar" lang={name === 'cjk' ? 'zh-Hans' : 'en'}>{item.char}</span>
            <span class="mono">{item.unicode}</span>
          </button>
        {/each}
      </div>
    {:else}
      <p class="plate__tabs plate__tabs--static">
        <span class="plate__tabChar">{specimen.char}</span>
        <span class="mono">{specimen.unicode}</span>
      </p>
    {/if}
    <span class="plate__source mono">{specimen.source}</span>
  </div>

  <!-- The drawing is an image until it is live; then the keyboard works on it,
       which is a thing you operate, so it is announced as one. The element
       that takes focus is this frame, not the picture inside it. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    bind:this={frame}
    class="plate__frame"
    role={live ? 'application' : 'presentation'}
    aria-label={live ? labels.figure : undefined}
    aria-describedby={live ? 'plate-hint' : undefined}
    tabindex={live ? 0 : undefined}
    onkeydown={live ? onkeydown : undefined}
  >
  <svg
    bind:this={svg}
    class="plate__canvas"
    viewBox="{data.frame[0]} {data.frame[1]} {data.frame[2]} {data.frame[3]}"
    role="img"
    aria-label={labels.figure}
    onpointerdown={live ? grab : undefined}
    onpointermove={live ? drag : undefined}
    onpointerup={live ? drop : undefined}
    onpointercancel={live ? drop : undefined}
  >
    <defs>
      <marker
        id="tick"
        viewBox="0 0 10 10"
        refX="5"
        refY="5"
        markerWidth="10"
        markerHeight="10"
        markerUnits="userSpaceOnUse"
        orient="auto"
      >
        <path class="tick" d="M3 1 L7 9" />
      </marker>
    </defs>
    <g class="plate__metrics">
      {#each specimen.guides as [name, value] (name)}
        <line
          class="dim {value === 0 ? 'dim--baseline' : ''}"
          x1={data.guideLeft}
          y1={-value}
          x2={data.guideRight}
          y2={-value}
        />
        <text class="dim__name" x={data.guideLeft} y={-value - 14} font-size={data.labelSize}>
          {name}
        </text>
        <text
          class="dim__value mono"
          x={data.guideRight}
          y={-value - 14}
          text-anchor="end"
          font-size={data.labelSize}
        >
          {value}
        </text>
      {/each}
    </g>

    <g bind:this={plate} transform="translate({shift} 0)">
      <line class="dim dim--edge" x1="0" y1={data.frame[1] + 40} x2="0" y2={baseline} />
      <line
        class="dim dim--edge"
        x1={specimen.advance}
        y1={data.frame[1] + 40}
        x2={specimen.advance}
        y2={baseline}
      />
      <g class="plate__advance">
        <line class="dim dim--arrow" x1="0" y1={baseline} x2={specimen.advance} y2={baseline} />
        <text
          class="dim__value mono"
          x={specimen.advance / 2}
          y={baseline - 14}
          text-anchor="middle"
          font-size={data.labelSize}
        >
          {specimen.advance}
        </text>
      </g>

      {#if ppem > 0}
        <g class="plate__pixels">
          {#each pixels as pixel, index (index)}
            <rect class="pixel" x={pixel.x} y={pixel.y} width={pixel.size} height={pixel.size} />
          {/each}
        </g>
      {/if}

      <path class="glyph" d={outline} />

      {#if ppem === 0}
        <g class="plate__handles">
          {#each specimen.handles as [anchor, control] (anchor + ':' + control)}
            {@const a = nodes[anchor]}
            {@const b = nodes[control]}
            {#if a && b}
              <line class="handle" x1={a.x} y1={-a.y} x2={b.x} y2={-b.y} />
            {/if}
          {/each}
        </g>
        <g class="plate__nodes">
          {#each nodes as node, index (index)}
            {#if node.t === 'off'}
              <circle
                class="node node--off {index === selected ? 'node--selected' : ''}"
                cx={node.x}
                cy={-node.y}
                r="7"
              />
            {:else if node.t === 'implied'}
              <circle class="node node--implied" cx={node.x} cy={-node.y} r="5" />
            {:else if node.s}
              <circle
                class="node node--smooth {index === selected ? 'node--selected' : ''}"
                cx={node.x}
                cy={-node.y}
                r="9"
              />
            {:else}
              <rect
                class="node {index === selected ? 'node--selected' : ''}"
                x={node.x - 8}
                y={-node.y - 8}
                width="16"
                height="16"
              />
            {/if}
            {#if live && node.t !== 'implied'}
              <circle class="hit" cx={node.x} cy={-node.y} r="30" data-index={index} />
            {/if}
          {/each}
        </g>
      {/if}
    </g>
  </svg>
  </div>

  <figcaption class="plate__foot">
    <dl class="gauge">
      <div><dt>{labels.point}</dt><dd class="mono">{readout}</dd></div>
      <div><dt>{labels.points}</dt><dd class="mono">{specimen.points}</dd></div>
      <div><dt>{labels.advance}</dt><dd class="mono">{specimen.advance}/{specimen.upem}</dd></div>
      <div>
        <dt>{labels.curves}</dt>
        <dd class="mono">
          {specimen.curveType === 'cubic' ? labels.cubic : labels.quadratic}
        </dd>
      </div>
    </dl>
    {#if live}
      <p class="plate__hint" id="plate-hint">{labels.hint}</p>
      <div class="plate__strike">
        <label class="mono" for="plate-ppem">{labels.strike}</label>
        <input
          id="plate-ppem"
          type="range"
          min="0"
          max={STRIKES.length - 1}
          step="1"
          bind:value={
            () => strike, (next) => (strike = typeof next === 'number' ? next : Number(next))
          }
        />
        <output class="mono" for="plate-ppem">{ppem > 0 ? `${ppem} ppem` : labels.off}</output>
        <button class="plate__tab" type="button" onclick={reset}>{labels.reset}</button>
      </div>
    {/if}
  </figcaption>
</figure>
