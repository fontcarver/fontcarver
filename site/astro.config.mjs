// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

// The site is served from the apex domain; canonical, og:url and the sitemap
// all need absolute addresses.
const site = process.env.FC_SITE ?? 'https://fontcarver.com';

export default defineConfig({
  site,
  integrations: [svelte()],
  trailingSlash: 'always',
  build: {
    format: 'directory',
    // The content security policy in public/_headers has no 'unsafe-inline',
    // so nothing may be inlined into the document: not a stylesheet, and not
    // a hydration bootstrap. Svelte components therefore render statically
    // here and are mounted by one bundled module (src/lib/enhance.ts), rather
    // than through a client: directive, which would emit an inline loader.
    inlineStylesheets: 'never',
  },
  devToolbar: { enabled: false },
});
