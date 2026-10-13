# Application icons

Every file in this directory except `app-icon.svg` is generated. `app-icon.svg`
is the logo, and the only one to edit or replace.

```sh
node app/node_modules/@tauri-apps/cli/tauri.js icon src-tauri/icons/app-icon.svg
```

Run it from the repository root — the CLI locates the project by finding
`src-tauri/tauri.conf.json` below the working directory — and commit everything
it rewrites: the desktop PNGs, `icon.icns`, `icon.ico`, the Microsoft Store
`Square*Logo.png` set and the `android/` and `ios/` launcher icons.

The generator rasterizes an SVG at its intrinsic `width`/`height`, so
`app-icon.svg` declares 1024 while keeping its 512-unit `viewBox`: the drawing
is resolution independent, and 1024 is the master size macOS (`ic10`) and
Windows (a 256-pixel `.ico` entry) want. A replacement logo should do the same.

The same artwork is the web favicon in `app/public/favicon.svg` and the site
favicon in `site/assets/img/favicon.svg`, at their natural 512 size.

`tauri.conf.json` ships only `32x32.png`, `128x128.png`, `128x128@2x.png`,
`icon.icns` and `icon.ico`; those are what reach the deb, rpm, AppImage, dmg and
NSIS packages, where the Linux bundle inspector also checks that the desktop
entry's icon is present.
