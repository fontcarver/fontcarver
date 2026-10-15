/*
 * The shape both languages fill. The English page and the Chinese page render
 * from the same components, so a string that exists in one and not the other
 * is a type error rather than a silent gap on the page.
 */

export interface Link {
  label: string;
  href: string;
}

export interface Key {
  combo: string[];
  label: string;
}

export interface TerminalLine {
  command: string;
  comment?: string;
}

export interface Content {
  lang: string;
  /** This page's own address, used for canonical and hreflang. */
  dir: string;
  other: { href: string; label: string; lang: string };
  meta: { title: string; description: string; ogTitle: string; ogDescription: string };
  nav: {
    sections: string;
    capabilities: string;
    formats: string;
    guide: string;
    download: string;
    github: string;
    toDark: string;
    toLight: string;
    skip: string;
  };
  hero: {
    title: string;
    lede: string;
    download: string;
    source: string;
    noteLead: string;
    note: string;
    noteLink: string;
  };
  drawing: {
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
    figure: string;
    hint: string;
  };
  capabilities: {
    title: string;
    lede: string;
    rows: { key: string; title: string; text: string }[];
  };
  formats: {
    title: string;
    lede: string;
    groups: { title: string; items: string[] }[];
    note: string;
    noteLink: string;
  };
  guide: {
    title: string;
    lede: string;
    steps: { title: string; text: string; keys: Key[] }[];
    terminal: TerminalLine[];
  };
  download: {
    title: string;
    lede: string;
    published: string;
    prerelease: string;
    checksums: string;
    releasePage: string;
    yourSystem: string;
    platforms: Record<'linux' | 'macos' | 'windows', { name: string; note: string }>;
    kinds: Record<'deb' | 'rpm' | 'appimage' | 'dmg' | 'nsis' | 'portable', string>;
    warningTitle: string;
    warning: string;
    verify: string;
    buildTitle: string;
    buildText: string;
    buildTerminal: TerminalLine[];
  };
  status: {
    title: string;
    lede: string;
    phases: { id: string; text: string; state: 'done' | 'active' }[];
    text: string;
    link: string;
  };
  footer: {
    about: string;
    aboutText: string;
    project: string;
    projectLinks: Link[];
    licence: string;
    licenceLinks: Link[];
    sister: string;
    sisterText: string;
    sisterLink: string;
    fine: string;
  };
  lost: { title: string; text: string; back: string };
}

/** One megabyte is 1,000,000 bytes here, as every download page states it. */
export function megabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}
