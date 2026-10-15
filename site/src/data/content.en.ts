import type { Content } from './content';

export const en: Content = {
  lang: 'en',
  dir: '/',
  other: { href: '/zh/', label: '中文', lang: 'zh-Hans' },
  meta: {
    title: 'FontCarver — a desktop font editor for curves and pixels',
    description:
      'FontCarver is an open-source desktop font editor with a Rust kernel. Outlines and bitmap strikes, Latin and CJK, and the whole OpenType layout stack in one document, for Linux, macOS and Windows.',
    ogTitle: 'FontCarver — a desktop font editor for curves and pixels',
    ogDescription:
      'Outlines and bitmap strikes, Latin and CJK, and the whole OpenType layout stack in one document. Open source, Rust kernel, Linux, macOS and Windows.',
  },
  nav: {
    sections: 'Sections',
    capabilities: 'Capabilities',
    formats: 'Formats',
    guide: 'Guide',
    download: 'Download',
    github: 'GitHub',
    toDark: 'Switch to dark theme',
    toLight: 'Switch to light theme',
    skip: 'Skip to content',
  },
  hero: {
    title: 'Draw outlines, edit pixels, ship OpenType.',
    lede: 'A desktop font editor with a Rust kernel. Outlines and bitmap strikes live in the same document, a 65,535-glyph CJK family gets the same tools as a single-master Latin face, and the tables you never touched come back out of the exporter unchanged.',
    download: 'Download',
    source: 'Source on GitHub',
    noteLead: 'is the first published build.',
    note: 'It is unsigned on every platform, and the native acceptance checklist in the release documentation is not finished. Treat it as an alpha and keep backups of the fonts you open.',
    noteLink: 'What the release was checked against',
  },
  drawing: {
    specimen: 'Specimen',
    reset: 'reset',
    point: 'point',
    points: 'points',
    advance: 'advance',
    curves: 'curves',
    strike: 'strike',
    off: 'off',
    none: '—',
    corner: 'corner',
    smooth: 'smooth',
    handle: 'handle',
    implied: 'implied',
    cubic: 'cubic',
    quadratic: 'quadratic',
    figure: 'A glyph outline with its points, metrics and pixel strike',
    hint: 'Drag a point. Or focus the drawing: arrow keys nudge the selected point one font unit, Shift ten, Page Up and Page Down step through the points. The strike is rasterised from the outline as it stands, so moving a point moves a pixel.',
  },
  capabilities: {
    title: 'Four things the editor refuses to treat as an afterthought.',
    lede: 'Most editors are built around one kind of font and grow the rest later. The document model here starts with all four.',
    rows: [
      {
        key: 'outline ⇄ strike',
        title: 'Curves and pixels, one document',
        text: 'Cubic and quadratic outline editing and bitmap-strike editing are the same document, not two applications. Trace pixels into outlines, rasterise outlines into a strike, then fix the strike by hand.',
      },
      {
        key: '65,535 glyphs',
        title: 'Latin and CJK, same tools',
        text: 'The glyph grid, search, batch operations and file input and output are built for 65,535-glyph fonts. Components, vertical metrics and vertical layout features are part of the model rather than a plug-in.',
      },
      {
        key: 'GSUB GPOS GDEF',
        title: 'OpenType layout you can read',
        text: 'GSUB, GPOS, GDEF, BASE, MATH, STAT, COLR and CPAL, edited as feature-file source or through the interface, compiled with fea-rs and fontc, with diagnostics located in your own source.',
      },
      {
        key: 'verbatim tables',
        title: 'Lossless where you did not touch anything',
        text: 'Tables the model does not represent are preserved through open and export, and glyphs you did not edit keep their hinting. Opening a font is not a lossy act.',
      },
    ],
  },
  formats: {
    title: 'The formats a working type office actually has on disk.',
    lede: 'Binaries, sources, bitmap strikes and layout sources, read and written by the same kernel.',
    groups: [
      {
        title: 'Outline binaries',
        items: ['TTF', 'OTF / CFF', 'CFF2', 'TTC', 'OTC', 'WOFF', 'WOFF2', 'Variable fonts'],
      },
      {
        title: 'Bitmap fonts and strikes',
        items: ['BDF', 'PCF', 'PSF', 'FNT', 'FON', 'OTB', 'EBDT / EBLC', 'CBDT / CBLC', 'sbix'],
      },
      {
        title: 'Sources',
        items: [
          'UFO 2',
          'UFO 3',
          'designspace',
          'Glyphs 2',
          'Glyphs 3',
          'SFD',
          'Type 1',
          'SVG',
          '.fcf',
        ],
      },
      {
        title: 'Layout and metrics',
        items: [
          'FEA',
          'GSUB',
          'GPOS',
          'GDEF',
          'BASE',
          'MATH',
          'STAT',
          'COLR / CPAL',
          'AFM',
          'PFM',
          'TTX',
        ],
      },
    ],
    note: 'This is the product target. Coverage is recorded format by format in the acceptance reports, and conversion stays conservative: anything unsupported or unproven keeps its original tables rather than being rewritten.',
    noteLink: 'Read the acceptance records',
  },
  guide: {
    title: 'A first session, start to finish.',
    lede: 'Four steps from an existing font to one you exported yourself. Keys are shown for Windows and Linux; on macOS, Ctrl is ⌘.',
    steps: [
      {
        title: 'Open a font',
        text: 'Open a TTF, OTF, UFO, Glyphs or SFD file, or a FontCarver document. What the model cannot represent is kept rather than dropped, so you can open a font just to look at it.',
        keys: [
          { combo: ['Ctrl', 'O'], label: 'open' },
          { combo: ['Ctrl', 'Shift', 'P'], label: 'command palette' },
        ],
      },
      {
        title: 'Edit outlines',
        text: 'Draw with the pen, cut with the knife, nudge nodes with the arrow keys. Smooth nodes keep their handles collinear, corner nodes do not, and every edit goes through one undo stack.',
        keys: [
          { combo: ['V'], label: 'select' },
          { combo: ['P'], label: 'pen' },
          { combo: ['B'], label: 'pencil' },
          { combo: ['E'], label: 'knife' },
          { combo: ['Ctrl', 'Z'], label: 'undo' },
        ],
      },
      {
        title: 'Kern, then add features',
        text: 'Set kerning in the gap between two glyphs in a text tab, with shared groups and per-master pairs. Write the rest as feature-file source and read the compiler’s diagnostics against your own lines.',
        keys: [
          { combo: ['T'], label: 'text tool' },
          { combo: ['Ctrl', 'I'], label: 'font info' },
        ],
      },
      {
        title: 'Export',
        text: 'Write a TTF or OTF back out. Glyphs you never edited keep their hinting, and the preserved tables are merged back in. Outline edits currently drop that glyph’s hinting; the status record says where that stands.',
        keys: [
          { combo: ['Ctrl', 'S'], label: 'save' },
          { combo: ['Ctrl', 'E'], label: 'export' },
        ],
      },
    ],
    terminal: [
      { command: 'fontcarver inspect DejaVuSans.ttf', comment: 'metadata and preserved tables' },
      { command: 'fontcarver check Source.fcf', comment: 'find problems, exit 1 on errors' },
      { command: 'fontcarver convert Source.glyphs Source.ufo' },
      { command: 'fontcarver build Source.fcf Source.otf --autohint' },
      { command: 'fontcarver shape Source.fcf "Hamburgefonstiv" --features liga,-kern' },
    ],
  },
  download: {
    title: 'Linux, macOS and Windows, from one Rust kernel.',
    lede: 'Every file below was built and checked on the system it targets, then published with the digest its own inspection recorded.',
    published: 'published',
    prerelease: 'pre-release',
    checksums: 'SHA256SUMS.txt',
    releasePage: 'Release page and notes',
    yourSystem: 'your system',
    platforms: {
      linux: { name: 'Linux', note: 'x86_64' },
      macos: { name: 'macOS', note: 'Apple silicon, no Intel build' },
      windows: { name: 'Windows', note: 'x64' },
    },
    kinds: {
      deb: 'Debian and Ubuntu',
      rpm: 'Fedora and other rpm distributions',
      appimage: 'any distribution with glibc at least as new as Ubuntu 24.04',
      dmg: 'disk image, drag to Applications',
      nsis: 'installer, English and Simplified Chinese',
      portable: 'portable, keeps its data beside the executable',
    },
    warningTitle: 'Nothing here is signed.',
    warning:
      'There is no Developer ID signature, no Apple notarization and no Authenticode signature, so macOS refuses the application on first launch and Windows shows a SmartScreen warning. Verify the digest first, then open it from the Finder with Control-click → Open, or choose More info → Run anyway.',
    verify: 'Verify a download against the release’s own checksum file:',
    buildTitle: 'Build from source',
    buildText:
      'You need Rust at the version pinned in rust-toolchain.toml, Node 24 or later, pnpm, and the Tauri system libraries. On Debian and Ubuntu those are libwebkit2gtk-4.1-dev, libgtk-3-dev, libayatana-appindicator3-dev and librsvg2-dev.',
    buildTerminal: [
      { command: 'git clone https://github.com/fontcarver/fontcarver.git' },
      { command: 'cd fontcarver' },
      { command: 'pnpm install' },
      { command: 'pnpm tauri dev', comment: 'run the editor' },
      { command: 'cargo test --workspace', comment: 'the Rust kernel’s tests' },
    ],
  },
  status: {
    title: 'Where the work actually stands.',
    lede: 'A phase is complete within the boundaries its acceptance report states, not exhaustively.',
    phases: [
      { id: 'P0', text: 'Shell, lossless round trip, glyph grid', state: 'done' },
      { id: 'P1', text: 'Outline editing, tools, export', state: 'done' },
      { id: 'P2', text: 'Kerning, editable OpenType layout, sources', state: 'done' },
      { id: 'P3', text: 'Bitmap strikes and bitmap formats', state: 'done' },
      { id: 'P4', text: 'Variable fonts, CJK construction, vertical', state: 'done' },
      { id: 'P5', text: 'Professional finish, hinting, packaging', state: 'active' },
    ],
    text: 'P5 is the current phase, and the published alpha is its first packaged build: the installers are verified by scripted install and removal on each platform, while interactive native editing, signing and upgrade behaviour are not. Everything open is tracked in the status record.',
    link: 'Read the status record',
  },
  footer: {
    about: 'About',
    aboutText:
      'A from-scratch font editor, dual licensed under Apache-2.0 and MIT. Built on the permissively licensed Rust font ecosystem: fontations, fontc, kurbo, fea-rs, harfrust, norad and tiny-skia.',
    project: 'Project',
    projectLinks: [
      { label: 'Repository', href: 'https://github.com/fontcarver/fontcarver' },
      { label: 'Releases', href: 'https://github.com/fontcarver/fontcarver/releases' },
      {
        label: 'Status',
        href: 'https://github.com/fontcarver/fontcarver/blob/main/docs/STATUS.md',
      },
      { label: 'Documentation', href: 'https://github.com/fontcarver/fontcarver/tree/main/docs' },
    ],
    licence: 'Licence',
    licenceLinks: [
      { label: 'Apache-2.0', href: 'https://github.com/fontcarver/fontcarver/blob/main/LICENSE-APACHE' },
      { label: 'MIT', href: 'https://github.com/fontcarver/fontcarver/blob/main/LICENSE-MIT' },
      {
        label: 'Third-party notices',
        href: 'https://github.com/fontcarver/fontcarver/blob/main/THIRD_PARTY.md',
      },
    ],
    sister: 'Sister project',
    sisterText:
      'FontPixel catalogues freely licensed bitmap fonts, with per-glyph measurements and previews you can type into.',
    sisterLink: 'fontpixel.com',
    fine: 'FontCarver is a from-scratch implementation, not a port. FontForge is studied as a behavioural and format reference only; FontCarver never links against, embeds or ships it, and is not affiliated with, endorsed by or sponsored by the FontForge project. The specimens on this page are drawn from Source Serif 4 and Noto Serif CJK SC, both under the SIL Open Font License.',
  },
  lost: {
    title: 'That address is not in the font.',
    text: 'The page you asked for does not exist here. The front page has the editor, the formats it reads and writes, and the current downloads.',
    back: 'Go to the front page',
  },
};
