# Windows Portable Build Implementation Plan (Plan D)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `FontCarver_<version>_x64-portable.zip` beside the NSIS installer: extract and run, with every byte of state kept in a `data` folder next to the executable.

**Architecture:** A marker file `PORTABLE.txt` beside the executable switches the app into portable mode. Portable mode redirects two stores: FontCarver's own config/autosave directory (through the existing `Api::configure_config_dir` seam) and the WebView's user-data directory, where the frontend keeps its `localStorage` state. Tauri forces a WebView data directory under `%LOCALAPPDATA%` and its config-file field accepts only relative paths resolved under that same root, so the only way to place it beside the executable is `WebviewWindowBuilder::data_directory(<absolute>)` in code. The main window therefore moves from config auto-creation to `WebviewWindowBuilder::from_config`, and workspace windows apply the same directory so all windows keep sharing one `localStorage`.

**Tech Stack:** Rust (src-tauri crate `fontcarver`, Tauri 2.11.5), Python 3.12 stdlib for the build and inspector.

**Spec:** `docs/superpowers/specs/2026-10-07-release-channels-design.md` §6

## Global Constraints

- No new dependencies. `tauri::webview_version` is re-exported from `tauri_runtime_wry`.
- Non-portable runs must behave exactly as today: no marker → no `data_directory`
  call → Tauri's own default applies to every window, as before.
- **All** webviews use the same data directory in a given run. A main window and a
  workspace window with different directories would get separate `localStorage`
  and silently stop sharing UI state.
- The WebView2-presence check and the zip itself only exist on Windows, which is not
  available here. Those parts are written to their documented contracts and are
  labelled unverified; nothing may claim Windows behaviour was observed.
- Do not touch `app/src/**` — the user has uncommitted work there, and this plan
  needs no frontend change.
- Run `python3 scripts/check_artifacts.py` before reporting each task complete.

## Review Focus

1. **A non-portable run must not change where anything is written.** The main window now comes from `from_config`; if that path accidentally sets a data directory, every existing user's localStorage appears to vanish on upgrade.
2. **A workspace window opened in portable mode must use the portable WebView directory.** Missing this leaves cross-window state split, which shows up as settings that "do not stick" in detached panels.
3. **The marker must be a file.** A directory named `PORTABLE.txt` must not switch modes.
4. **The executable path must be resolved before looking for the marker.** A shortcut or symlink to the exe must find the marker beside the real file, not beside the link.
5. **A missing WebView2 runtime must produce a readable message, not a blank window or a crash.** The portable zip has no installer to bootstrap it.

---

### Task 1: Portable-mode resolution

**Files:**
- Create: `src-tauri/src/portable.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod portable;`)

**Interfaces:**
- Produces: `portable::MARKER: &str = "PORTABLE.txt"`;
  `portable::root(executable: &Path) -> Option<PathBuf>` returning `<exe dir>/data`
  only when `<exe dir>/PORTABLE.txt` is a regular file;
  `portable::config_dir(root) -> PathBuf` (`root/config`);
  `portable::webview_dir(root) -> PathBuf` (`root/webview`). Tasks 2 and 3 use all four.

- [ ] **Step 1: Write the failing test** — in `src-tauri/src/portable.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_file_beside_the_executable_selects_a_data_root() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::write(dir.path().join(MARKER), b"portable").unwrap();
        assert_eq!(root(&exe), Some(dir.path().join("data")));
    }

    #[test]
    fn no_marker_means_an_installed_build() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert_eq!(root(&exe), None);
    }

    #[test]
    fn a_directory_named_like_the_marker_does_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::create_dir(dir.path().join(MARKER)).unwrap();
        assert_eq!(root(&exe), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_the_executable_finds_the_marker_beside_the_real_file() {
        let real = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let exe = real.path().join("fontcarver-desktop");
        std::fs::write(&exe, b"\x7fELF").unwrap();
        std::fs::write(real.path().join(MARKER), b"portable").unwrap();
        let link = elsewhere.path().join("fontcarver-desktop");
        std::os::unix::fs::symlink(&exe, &link).unwrap();
        let resolved = std::fs::canonicalize(&link).unwrap();
        assert_eq!(root(&resolved), Some(real.path().canonicalize().unwrap().join("data")));
    }

    #[test]
    fn stores_live_in_separate_subdirectories() {
        let root = Path::new("/portable/data");
        assert_eq!(config_dir(root), root.join("config"));
        assert_eq!(webview_dir(root), root.join("webview"));
    }
}
```

`tempfile` must already be a dev-dependency of the crate; if not, add
`tempfile = { workspace = true }` under `[dev-dependencies]` in `src-tauri/Cargo.toml`
(it is a workspace dependency, so no new crate enters the graph).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fontcarver --lib portable 2>&1 | tail -15`
Expected: FAIL — `cannot find function `root`` / `cannot find value `MARKER``

- [ ] **Step 3: Write minimal implementation** — top of `src-tauri/src/portable.rs`:

```rust
//! Portable mode: state beside the executable instead of in the user's profile.
//!
//! The portable zip ships `PORTABLE.txt` next to the executable. Its presence is
//! the switch; an installed build never has it.

use std::path::{Path, PathBuf};

pub(crate) const MARKER: &str = "PORTABLE.txt";

/// The data root for a portable run, or `None` for an installed one.
///
/// Callers pass a resolved path: a shortcut or link must find the marker beside
/// the real executable, not beside the link.
pub(crate) fn root(executable: &Path) -> Option<PathBuf> {
    let dir = executable.parent()?;
    dir.join(MARKER).is_file().then(|| dir.join("data"))
}

pub(crate) fn config_dir(root: &Path) -> PathBuf {
    root.join("config")
}

pub(crate) fn webview_dir(root: &Path) -> PathBuf {
    root.join("webview")
}
```

and add `mod portable;` beside the other module declarations in `src-tauri/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p fontcarver --lib portable 2>&1 | tail -15`
Expected: PASS, 5 tests on Linux (4 on Windows, where the link test is cfg'd out)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/portable.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(desktop): resolve portable mode from a marker beside the executable"
```

---

### Task 2: Redirect both stores in portable mode

**Files:**
- Modify: `src-tauri/src/lib.rs` — `launch()`
- Modify: `src-tauri/src/windows.rs` — `open_workspace_window`
- Modify: `src-tauri/tauri.conf.json` — `app.windows[0]` gains `"create": false`

**Interfaces:**
- Consumes: Task 1's `portable::{root, config_dir, webview_dir}`.
- Produces: managed state `portable::Root(Option<PathBuf>)` that windows.rs reads.

- [ ] **Step 1: Write the failing test** — append to `src-tauri/src/portable.rs` tests:

```rust
    #[test]
    fn the_main_window_is_created_in_code_so_its_data_directory_can_be_set() {
        // Tauri resolves a config-file data_directory under %LOCALAPPDATA% and
        // rejects absolute paths, so the portable directory can only be applied
        // to a window built in code. The config must therefore not auto-create it.
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let main = config["app"]["windows"].as_array().unwrap().iter()
            .find(|window| window["label"] == "main").unwrap();
        assert_eq!(main["create"], serde_json::Value::Bool(false));
    }

    #[test]
    fn webview_directory_is_only_set_in_portable_mode() {
        assert_eq!(webview_directory(&Root(None)), None);
        let root = PathBuf::from("/p/data");
        assert_eq!(webview_directory(&Root(Some(root.clone()))), Some(root.join("webview")));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fontcarver --lib portable 2>&1 | tail -15`
Expected: FAIL — `create` is null, and `Root` / `webview_directory` do not exist

- [ ] **Step 3: Write minimal implementation**

Append to `src-tauri/src/portable.rs`:

```rust
/// This run's portable data root, managed so every window can reach it.
pub(crate) struct Root(pub(crate) Option<PathBuf>);

/// The WebView data directory to apply, or `None` to leave Tauri's default.
///
/// Every window in a run must agree, or windows stop sharing localStorage.
pub(crate) fn webview_directory(root: &Root) -> Option<PathBuf> {
    root.0.as_deref().map(webview_dir)
}
```

In `tauri.conf.json`, add `"create": false` to the `main` window object.

In `launch()`, resolve the root before anything touches the config directory, and
configure it before `start_autosave`:

```rust
    let portable = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| portable::root(&exe));
    let api = Arc::new(Api::new(Arc::new(Session::new())));
    if let Some(root) = &portable {
        if let Err(error) = api.configure_config_dir(portable::config_dir(root)) {
            tracing::warn!(%error, "could not use the portable config directory");
        }
    }
    if let Err(error) = api.start_autosave() {
```

Manage the root and build the main window in `setup`, after the event thread:

```rust
        .manage(portable::Root(portable.clone()))
```

```rust
            let config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or("tauri.conf.json defines no main window")?;
            let mut main = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?;
            if let Some(directory) = portable::webview_directory(&app.state::<portable::Root>()) {
                main = main.data_directory(directory);
            }
            main.build()?;
            Ok(())
```

`setup` needs `use tauri::Manager;` in scope for `app.state`.

In `windows.rs::open_workspace_window`, apply the same directory:

```rust
    let mut builder = WebviewWindowBuilder::new(&app, target.label, WebviewUrl::App(target.url.into()))
        .title("FontCarver")
        .inner_size(width, 850.0)
        .min_inner_size(min_width, 600.0);
    if let Some(directory) = crate::portable::webview_directory(&app.state::<crate::portable::Root>()) {
        builder = builder.data_directory(directory);
    }
    let window = builder.build().map_err(|error| error.to_string())?;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p fontcarver 2>&1 | tail -15`
Expected: PASS — all crate tests, including the existing windows.rs and cli.rs ones

- [ ] **Step 5: Verify the main window still starts in a normal run**

The main window's creation path changed on every platform, so a unit test is not
enough. Build and launch with the repository's own smoke check (this briefly opens
a FontCarver window on the desktop):

Run: `cargo build --release -p fontcarver 2>&1 | tail -3`
Run: `python3 scripts/smoke_release_native.py --executable target/release/fontcarver-desktop --launch gui --report .scratch/plan-d/smoke.json --seconds 8`
Expected: exit 0, and the report shows the process stayed alive for the window
Run: `python3 scripts/check_artifacts.py`
Expected: `PASS`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/windows.rs src-tauri/src/portable.rs src-tauri/tauri.conf.json
git commit -m "feat(desktop): keep config and WebView data beside a portable executable"
```

---

### Task 3: Refuse clearly when WebView2 is missing

**Files:**
- Modify: `src-tauri/src/portable.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `portable::webview_missing_message(detail: &str) -> String`.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn a_missing_webview_runtime_is_explained_in_plain_words() {
        let message = webview_missing_message("class not registered");
        assert!(message.contains("WebView2"));
        assert!(message.contains("https://developer.microsoft.com/microsoft-edge/webview2/"));
        assert!(message.contains("class not registered"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p fontcarver --lib portable 2>&1 | tail -10`
Expected: FAIL — `cannot find function `webview_missing_message``

- [ ] **Step 3: Write minimal implementation**

```rust
/// The text shown when the system WebView runtime cannot be found.
///
/// The portable zip has no installer to bootstrap WebView2, so on a Windows host
/// without it this is the only feedback the user gets.
pub(crate) fn webview_missing_message(detail: &str) -> String {
    format!(
        "FontCarver needs the Microsoft Edge WebView2 Runtime, which was not found.\n\n\
         Windows 11 includes it. On Windows 10, install it from \
         https://developer.microsoft.com/microsoft-edge/webview2/ and start FontCarver again.\n\n\
         Detail: {detail}"
    )
}
```

In `launch()`, before the builder, on Windows only:

```rust
    #[cfg(windows)]
    if let Err(error) = tauri::webview_version() {
        let message = portable::webview_missing_message(&error.to_string());
        tracing::error!("{message}");
        rfd_or_stderr(&message);
        return Err(Error::from(std::io::Error::other(message)));
    }
```

Use whatever native message path the crate already has for fatal startup errors
(`error.rs` / `guarded`); if none can show a dialog before the webview exists,
print to stderr and return the error, and record that limitation in the ledger.
**This branch cannot run on Linux and is unverified.**

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p fontcarver 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/portable.rs src-tauri/src/lib.rs
git commit -m "feat(desktop): explain a missing WebView2 runtime instead of failing silently"
```

---

### Task 4: Build and inspect the portable zip

**Files:**
- Modify: `scripts/build_release.py` — produce the zip after an `nsis` build
- Modify: `scripts/check_release_bundle.py` — `portable` kind
- Test: `scripts/test_build_release.py`, `scripts/test_check_release_bundle.py`

**Interfaces:**
- Produces: `build_release.build_portable_zip(executable: Path, licenses: Path, version: str, output_dir: Path) -> Path`;
  `check_release_bundle.SUFFIXES['portable'] == '-portable.zip'`, `HOSTS['portable'] == 'win32'`.

- [ ] **Step 1: Write the failing tests**

In `scripts/test_build_release.py`:

```python
    def test_portable_zip_has_one_versioned_root_with_marker_and_licenses(self):
        import zipfile
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exe = root / "fontcarver-desktop.exe"
            exe.write_bytes(b"MZ")
            licenses = root / "licenses"
            licenses.mkdir()
            (licenses / "DEPENDENCIES.json").write_text("{}")
            archive = build_release.build_portable_zip(exe, licenses, "0.1.0", root / "out")
            self.assertEqual(archive.name, "FontCarver_0.1.0_x64-portable.zip")
            with zipfile.ZipFile(archive) as zip_file:
                names = zip_file.namelist()
            top = {name.split("/", 1)[0] for name in names}
            self.assertEqual(top, {"FontCarver-0.1.0"})
            self.assertIn("FontCarver-0.1.0/fontcarver-desktop.exe", names)
            self.assertIn("FontCarver-0.1.0/PORTABLE.txt", names)
            self.assertIn("FontCarver-0.1.0/licenses/LICENSE-MIT", names)
            self.assertIn("FontCarver-0.1.0/licenses/THIRD_PARTY.md", names)
            self.assertIn("FontCarver-0.1.0/licenses/dependencies/DEPENDENCIES.json", names)
```

In `scripts/test_check_release_bundle.py`:

```python
    def test_portable_zip_is_a_windows_artifact_matched_by_name(self):
        self.assertEqual(verifier.HOSTS['portable'], 'win32')
        self.assertEqual(verifier.SUFFIXES['portable'], '-portable.zip')
        bundle = self.root / 'bundle'
        (bundle / 'portable').mkdir(parents=True)
        archive = bundle / 'portable/FontCarver_0.1.0_x64-portable.zip'
        archive.write_bytes(b'PK')
        self.assertEqual(verifier.discover(bundle, ['portable']), [('portable', archive)])
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 scripts/test_build_release.py 2>&1 | tail -3; python3 scripts/test_check_release_bundle.py 2>&1 | tail -3`
Expected: FAIL — `build_portable_zip` and `HOSTS['portable']` do not exist

- [ ] **Step 3: Write minimal implementation**

`scripts/build_release.py`:

```python
PORTABLE_NOTE = """FontCarver portable build

This folder runs without installing. While PORTABLE.txt sits beside
fontcarver-desktop.exe, FontCarver keeps its settings, autosave and browser
storage in the "data" folder here instead of in your user profile. Delete the
folder to remove everything.

Requires the Microsoft Edge WebView2 Runtime (included with Windows 11).
"""


def build_portable_zip(executable: Path, licenses: Path, version: str, output_dir: Path) -> Path:
    """Zip a Windows build under one versioned root so extraction cannot scatter files."""
    import zipfile
    root = f"FontCarver-{version}"
    output_dir.mkdir(parents=True, exist_ok=True)
    archive = output_dir / f"FontCarver_{version}_x64-portable.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zip_file:
        zip_file.write(executable, f"{root}/fontcarver-desktop.exe")
        zip_file.writestr(f"{root}/PORTABLE.txt", PORTABLE_NOTE)
        for name in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY.md"):
            zip_file.write(ROOT / name, f"{root}/licenses/{name}")
        for path in sorted(licenses.rglob("*")):
            if path.is_file():
                zip_file.write(path, f"{root}/licenses/dependencies/{path.relative_to(licenses).as_posix()}")
    return archive
```

After the Tauri build, when `"nsis" in bundles`, call it with
`target / "release/fontcarver-desktop.exe"`, the scratch `licenses` directory the
helper already prepares, the validated version, and `bundle_dir / "portable"`; then
add `"portable"` to the list passed to `check_release_bundle.py --expected`.

`scripts/check_release_bundle.py`: add `'portable': 'win32'` to `HOSTS` and
`'portable': '-portable.zip'` to `SUFFIXES`; in `extracted()`, a `portable` branch
that unpacks with `zipfile`, rejects members escaping the root, and yields the single
top-level directory; `inspect_payload` treats `portable` like `nsis` (PE executable
named `fontcarver-desktop.exe`) and additionally requires `PORTABLE.txt` at the root.

- [ ] **Step 4: Run tests to verify they pass**

Run: `for t in scripts/test_*.py; do python3 "$t" 2>&1 | tail -1; done`
Expected: every module OK

- [ ] **Step 5: Commit**

```bash
git add scripts/build_release.py scripts/check_release_bundle.py scripts/test_build_release.py scripts/test_check_release_bundle.py
git commit -m "feat(release): build and inspect a Windows portable zip"
```

---

### Task 5: Workflow and documentation

**Files:** `.github/workflows/release.yml` (upload `**/*-portable.zip`), `docs/install.md`
(a Windows "portable" row and steps), `docs/releasing.md` (the artifact set and that
portable behaviour is unverified on Windows).

- [ ] **Step 1:** add the upload glob, the install-table row and the portable steps.
- [ ] **Step 2:** `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` → no error; full script suite OK; `check_artifacts.py` PASS.
- [ ] **Step 3: Commit** `docs(release): document the Windows portable build`.
