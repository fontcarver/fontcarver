/*
 * Everything the page does after it has already been read.
 *
 * Astro bundles this to one module with a src, because the content security
 * policy in public/_headers allows no inline script. It mounts the live
 * drawing over the static one Astro rendered, remembers the theme, and moves
 * the visitor's own platform to the front of the download list. Each part is
 * an enhancement: the page is complete and readable without any of it.
 */

import { mount } from 'svelte';
import GlyphDrawing from '../components/GlyphDrawing.svelte';
import { specimens } from '../data/specimens';

const root = document.documentElement;

/* -------------------------------------------------------------- drawing */

const stage = document.getElementById('stage');
if (stage?.dataset.labels) {
  const labels = JSON.parse(stage.dataset.labels);
  // The static drawing and the live one draw the same glyph from the same
  // data, so replacing one with the other changes nothing on screen until
  // somebody touches it.
  stage.replaceChildren();
  mount(GlyphDrawing, { target: stage, props: { data: specimens, labels, live: true } });
}

/* ---------------------------------------------------------------- theme */

const toggle = document.getElementById('theme-toggle');
const label = document.getElementById('theme-toggle-label');

function describe(): void {
  if (!label) return;
  const { toLight = '', toDark = '' } = label.dataset;
  label.textContent = root.dataset.theme === 'dark' ? toLight : toDark;
}

if (toggle) {
  describe();
  toggle.addEventListener('click', () => {
    root.dataset.theme = root.dataset.theme === 'dark' ? 'light' : 'dark';
    try {
      window.localStorage.setItem('fontcarver-theme', root.dataset.theme ?? 'light');
    } catch {
      // Not being able to remember the choice is not a reason to refuse it.
    }
    describe();
  });
}

/* ------------------------------------------------------------- platform */

function platform(): string | null {
  const data = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData;
  const haystack = `${data?.platform ?? ''} ${navigator.userAgent ?? ''}`.toLowerCase();
  if (haystack.includes('mac')) return 'macos';
  if (haystack.includes('win')) return 'windows';
  // Android reports Linux too, and a phone is not a desktop download.
  if (haystack.includes('android')) return null;
  if (haystack.includes('linux') || haystack.includes('x11')) return 'linux';
  return null;
}

const here = platform();
if (here) {
  const block = document.querySelector<HTMLElement>(`[data-platform="${here}"]`);
  const list = document.getElementById('platforms');
  if (block && list) {
    block.dataset.promoted = 'true';
    const badge = block.querySelector<HTMLElement>('[data-badge]');
    if (badge) badge.hidden = false;
    if (list.firstElementChild !== block) list.insertBefore(block, list.firstElementChild);
  }
}
