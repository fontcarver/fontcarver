# Installing FontCarver

FontCarver is in development. Native build artifacts are for evaluation; a
successful package build alone does not establish desktop workflow acceptance.
See [release verification](releasing.md) for the build process and the remaining
platform checks. These instructions do not announce a published release.

## Choose a package

Use an artifact built for your operating system and CPU architecture. Keep the
package verification report with the download and compare its SHA-256 digest
before installation. A matching digest detects an incomplete or changed download;
it does not establish the identity of an unsigned publisher.

| System              | Package     | Installation                                         |
| ------------------- | ----------- | ---------------------------------------------------- |
| Debian/Ubuntu Linux | `.deb`      | Installed and removed by the system package manager  |
| Linux               | `.AppImage` | Executable file that you keep in a directory you own |
| macOS               | `.dmg`      | Disk image containing `FontCarver.app`               |
| Windows             | `.msi`      | Windows Installer package                            |

Linux compatibility depends on the build machine's system libraries. AppImage
does not guarantee compatibility with older glibc versions. The release build
matrix and package report identify the build baseline; other distributions need
their own verification. See [Tauri's AppImage limitations](https://v2.tauri.app/distribute/appimage/#limitations).

The desktop program is `fontcarver-desktop`. The separate `fontcarver` command-line
tool is built from `crates/fc-cli`; it is not installed by these desktop packages.

## Linux

For a `.deb`, replace the example filename with the package you downloaded:

```sh
dpkg-deb --field ./FontCarver_VERSION_ARCH.deb Package Version Architecture Depends
sudo apt install ./FontCarver_VERSION_ARCH.deb
```

APT resolves the package's declared runtime dependencies. Start FontCarver from
the application menu, or run `fontcarver-desktop` from a terminal. To remove it,
use `sudo apt remove PACKAGE_NAME`, substituting the `Package` value printed by
the first command. This removes the application package while retaining your
documents and user configuration.

The 0.1.0 x86_64 build names the file `FontCarver_0.1.0_amd64.deb` and the
package `font-carver`, so its removal command is `sudo apt remove font-carver`.
It declares `libwebkit2gtk-4.1-0, libgtk-3-0`, which between them pull in every
library the executable needs, and it installs:

| Path                             | Contents                                         |
| -------------------------------- | ------------------------------------------------ |
| `/usr/bin/fontcarver-desktop`    | the application                                  |
| `/usr/share/applications`        | the menu entry                                   |
| `/usr/share/icons/hicolor`       | 32, 128 and 256@2 pixel icons                    |
| `/usr/lib/FontCarver/licenses`   | both project licenses and dependency attribution |
| `/usr/lib/FontCarver/INSTALL.md` | a copy of this page                              |

From the next build onwards the package also installs the Debian copyright
document at `/usr/share/doc/font-carver/copyright`. The first 0.1.0 package
predates it and carries its licenses only under `/usr/lib/FontCarver/licenses`.

On Ubuntu 24.04 and other releases that completed the 64-bit time transition,
`libgtk-3-0` is satisfied by `libgtk-3-0t64`, which provides that name.

For an AppImage, put it in a directory you own and make that one file executable:

```sh
chmod u+x ./FontCarver_VERSION_ARCH.AppImage
./FontCarver_VERSION_ARCH.AppImage
```

The AppImage can be launched without installing a system package. If the runtime
reports a missing FUSE dependency, follow your distribution's AppImage/FUSE
instructions. Remove the AppImage file to uninstall this copy. A launcher added
manually to your desktop or application menu must be removed separately.

The AppImage carries its own copy of the GTK, WebKitGTK and related libraries,
which is why the 0.1.0 x86_64 image is about 96 MB against the deb's 23 MB. Those
bundled libraries keep the licenses of the distributions they came from, and
their notices are not yet complete inside the image, so the AppImage is an
evaluation artifact rather than a redistributable one. Prefer the deb where your
system can use it.

FontCarver needs a graphical session. Started without a reachable display it
stops while initialising its window toolkit rather than printing a diagnostic,
and it currently has no `--version` or `--help` output, so a terminal launch is
not a way to check an installation without a desktop.

## macOS

The bundle is configured for macOS 11.0 or newer. That deployment target is a
build setting; native compatibility and installation acceptance remain pending.

Open the `.dmg` for your Mac's architecture and copy `FontCarver.app` into
Applications. Eject the disk image and launch the installed app from Applications.
Quit the app before replacing an existing copy. To uninstall, quit FontCarver and
move that application bundle to the Trash; saved fonts and preferences are kept.

The current build workflow does not provide a verified Developer ID signature
or Apple notarization. macOS may block downloaded evaluation artifacts. Signed
distribution and installation acceptance are pending; do not treat an unsigned
CI artifact as an accepted public release.

## Windows

Run the `.msi` for your Windows architecture and follow the installer. Launch
FontCarver from its Start menu entry. Microsoft Edge WebView2 is required; the
installer’s configured bootstrapper may need network access if the runtime is
missing. See [Tauri's Windows runtime installation options](https://v2.tauri.app/distribute/windows-installer/#webview2-installation-options).

Separate English (`en-US`) and Simplified Chinese (`zh-CN`) MSI variants are
built. The installer rejects an older version installed over a newer one.

To uninstall, quit FontCarver and use **Settings → Apps → Installed apps →
FontCarver → Uninstall**. Authenticode signing and native Windows installation
acceptance are pending; the current CI package is an unsigned evaluation artifact.

## Your documents and preferences

Save work as `.fcf` before an upgrade and keep backups of source fonts. Installing
the editor does not install your exported fonts into the operating system.
Documents and exports are stored at the paths you choose.

The default configuration directory contains preferences, recent files and
recovery snapshots in its `autosave` subdirectory:

| System  | Default configuration directory                                               |
| ------- | ----------------------------------------------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME/fontcarver` when absolute; otherwise `~/.config/fontcarver` |
| macOS   | `~/Library/Application Support/fontcarver`                                    |
| Windows | Roaming AppData, normally `%APPDATA%\fontcarver`                              |

`FC_CONFIG_DIR` overrides this location. On first launch, a legacy `fontcutter`
configuration directory is moved to `fontcarver` only if the new directory does
not exist. An explicit `FC_CONFIG_DIR` is used as supplied.

Removing the application leaves this data in place. For a settings reset, quit
FontCarver and move the actual configuration directory to a backup location.
Preserve recovery snapshots until any unsaved work has been recovered. Browser
or system-webview storage can also retain window/UI state independently of this
directory; `FC_CONFIG_DIR` does not redirect that storage.

## Report an installation problem

Include the artifact filename and digest, operating system/version, CPU
architecture and the exact error. For Linux packages, also include the `Package`,
`Version`, `Architecture` and `Depends` fields shown above. A terminal launch may
provide useful diagnostics. Do not attach private font files or recovery data
unless you intend to share them.
