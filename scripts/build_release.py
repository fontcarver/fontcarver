#!/usr/bin/env python3
"""Build and inspect native installers without publishing or system installation.

Requires Python 3.11+, pinned Rust/pnpm, frontend dependencies and Tauri's native
system prerequisites. Use the same Cargo target directory as development.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

from check_release_bundle import SUFFIXES, VerificationError, discover

ROOT = Path(__file__).resolve().parents[1]


PORTABLE_NOTE = """FontCarver portable build

This folder runs without installing. While PORTABLE.txt sits beside
fontcarver-desktop.exe, FontCarver keeps its settings, autosave and browser
storage in the "data" folder here instead of in your user profile. Delete the
folder to remove everything.

Requires the Microsoft Edge WebView2 Runtime, which Windows 11 includes.
"""


def build_portable_zip(executable: Path, licenses: Path, version: str, output_dir: Path) -> Path:
    """Zip a Windows build under one versioned folder, laid out like the installer.

    The layout mirrors what the NSIS installer writes, because the payload
    inspector locates notices by that layout: the generated licenses directory
    already holds the project licenses beside DEPENDENCIES.json, and INSTALL.md
    sits at the root above it.
    """
    import zipfile
    folder = f"FontCarver-{version}"
    output_dir.mkdir(parents=True, exist_ok=True)
    for stale in output_dir.glob("*-portable.zip"):
        stale.unlink()
    archive = output_dir / f"FontCarver_{version}_x64-portable.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zip_file:
        zip_file.write(executable, f"{folder}/fontcarver-desktop.exe")
        zip_file.writestr(f"{folder}/PORTABLE.txt", PORTABLE_NOTE)
        zip_file.write(ROOT / "docs/install.md", f"{folder}/INSTALL.md")
        for name in ("LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY.md"):
            zip_file.write(ROOT / name, f"{folder}/licenses/{name}")
        for path in sorted(licenses.rglob("*")):
            if path.is_file():
                zip_file.write(path, f"{folder}/licenses/dependencies/"
                               f"{path.relative_to(licenses).as_posix()}")
    return archive


def native_bundles(system: str) -> list[str]:
    """Return only the installer formats maintained for a native platform."""
    bundles = {"Linux": ["deb", "rpm"], "Darwin": ["dmg"], "Windows": ["nsis"]}
    if system not in bundles:
        raise ValueError(f"unsupported release build platform: {system}")
    return bundles[system]


def allowed_bundles(system: str) -> list[str]:
    """Formats --bundles accepts: the native defaults plus opt-in ones.

    AppImage is opt-in rather than default. Its payload gate blocks every image
    until notices are written at build time, and one blocked artifact fails the
    whole run, so a default AppImage would stop deb and rpm from shipping too.
    """
    return native_bundles(system) + (["appimage"] if system == "Linux" else [])


def validate_versions(cargo: str, frontend: str, tauri: str) -> str:
    """Reject mismatched package versions and versions a native installer cannot encode."""
    if len({cargo, frontend, tauri}) != 1:
        raise ValueError(f"version mismatch: Cargo={cargo}, frontend={frontend}, Tauri={tauri}")
    # NSIS writes major.minor.patch.0 into VIProductVersion and keeps a SemVer
    # pre-release only in names; build metadata (+...) has no portable encoding.
    identifier = r"(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)"
    match = re.fullmatch(rf"(\d+)\.(\d+)\.(\d+)(?:-{identifier}(?:\.{identifier})*)?", cargo)
    if not match:
        raise ValueError("release version must be major.minor.patch with an optional SemVer pre-release")
    if any(int(value) > limit for value, limit in zip(match.groups(), [255, 255, 65535])):
        raise ValueError("release version exceeds native installer version limits")
    return cargo


def validate_target_environment(env: dict[str, str], configs: list[Path] | None = None) -> None:
    """Keep Cargo output paths consistent with the native bundle inspector."""
    if env.get("CARGO_BUILD_TARGET"):
        raise ValueError("unset CARGO_BUILD_TARGET for this native-only build entry point")
    for path in configs or []:
        if path.is_file() and tomllib.loads(path.read_text(encoding="utf-8")).get("build", {}).get("target"):
            raise ValueError(f"remove the Cargo build.target override for a native build: {path}")


def run(command: list[str], env: dict[str, str]) -> None:
    """Run an explicit argument vector from the repository root."""
    print("+ " + " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def artifact_snapshot(bundle: Path, bundles: list[str]) -> dict[str, tuple[int, int, int]]:
    """Remember existing candidates so a successful build cannot accept stale files."""
    wanted = tuple(SUFFIXES[kind] for kind in bundles)
    return {str(path): (path.stat().st_mtime_ns, path.stat().st_ctime_ns, path.stat().st_size)
            for path in bundle.rglob("*") if path.is_file()
            and path.name.lower().endswith(wanted)}


def validate_built_artifacts(bundle: Path, bundles: list[str], before: dict) -> None:
    """Require exactly the configured fresh artifacts, retaining old files on failure."""
    artifacts = discover(bundle, bundles)
    for kind in bundles:
        selected = [path for artifact_kind, path in artifacts if artifact_kind == kind]
        if len(selected) != 1:
            raise ValueError(f"expected 1 fresh {kind} candidate, found {len(selected)}; "
                             "move stale candidates out of the bundle directory before rebuilding")
    after = artifact_snapshot(bundle, bundles)
    stale = [path for path, value in after.items() if before.get(path) == value]
    if stale:
        raise ValueError("build left unchanged installer candidates: " + ", ".join(stale))


def source_state() -> dict:
    """Fingerprint all non-ignored inputs to the application and release tooling."""
    roots = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "pnpm-workspace.yaml",
             "pnpm-lock.yaml", "package.json", ".npmrc", ".cargo",
             "crates", "src-tauri", "app", "scripts", "docs/install.md",
             "THIRD_PARTY.md", "LICENSE-MIT", "LICENSE-APACHE"]
    output = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others",
                                      "--exclude-standard", "--", *roots], cwd=ROOT)
    digest = hashlib.sha256()
    names = sorted(set(output.decode().split("\0")) - {""})
    for name in names:
        path = ROOT / name
        digest.update(name.encode() + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest() if path.is_file() else b"deleted")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    return {"revision": revision, "input_sha256": digest.hexdigest(), "input_files": len(names)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundles", help="comma-separated native formats; defaults to this platform")
    parser.add_argument("--min-free-gib", type=float, default=20.0,
                        help="workspace free-space floor; ephemeral CI may use 5")
    args = parser.parse_args()
    env = dict(os.environ)
    scratch_root = ROOT / ".scratch/p5-release"
    scratch_root.mkdir(parents=True, exist_ok=True)
    scratch = Path(tempfile.mkdtemp(prefix="build-", dir=scratch_root))
    temporary = scratch / "tmp"
    temporary.mkdir(exist_ok=True)
    env.update(TMPDIR=str(temporary), TMP=str(temporary), TEMP=str(temporary),
               TAURI_APP_PATH=str(ROOT / "src-tauri"), TAURI_FRONTEND_PATH=str(ROOT / "app"))
    target = Path(env.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    if not target.is_absolute():
        target = ROOT / target
    env["CARGO_TARGET_DIR"] = str(target)
    budget = [sys.executable, "scripts/check_artifacts.py", "--min-free-gib", str(args.min_free_gib)]
    result = 1
    try:
        run(budget, env)
        cargo_home = Path(env.get("CARGO_HOME", str(Path.home() / ".cargo")))
        config_dirs = [cargo_home] + [path / ".cargo" for path in (ROOT / "src-tauri", *(ROOT / "src-tauri").parents)]
        validate_target_environment(env, [directory / name for directory in config_dirs
                                          for name in ("config", "config.toml")])
        native = native_bundles(platform.system())
        bundles = [part.strip().lower() for part in args.bundles.split(",")] if args.bundles else native
        allowed = allowed_bundles(platform.system())
        if not bundles or any(bundle not in allowed for bundle in bundles):
            raise ValueError(f"native bundle formats on this platform are: {', '.join(native)}")
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        frontend = json.loads((ROOT / "app/package.json").read_text(encoding="utf-8"))
        config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        version = validate_versions(manifest["workspace"]["package"]["version"],
                                    frontend["version"], config["version"])
        pnpm = shutil.which("pnpm")
        if pnpm is None:
            raise ValueError("pnpm is not installed; use the version pinned in package.json")
        source = source_state()
        # The license inventory covers all locked Cargo targets, so a clean
        # native runner needs the other platforms' package manifests as well.
        run(["cargo", "fetch", "--locked"], env)
        run([sys.executable, "scripts/check_release_licenses.py", "--output-dir",
             str(scratch / "licenses")], env)
        run(["cargo", "run", "--locked", "-p", "fc-api", "--bin", "gen_ts"], env)
        run(budget, env)
        # An absolute mapping is portable and avoids generated files in static
        # tauri.conf.json, so ordinary development needs no release preparation.
        override = {"bundle": {"resources": {
            str(scratch / "licenses"): "licenses/dependencies",
        }}}
        override_path = scratch / "tauri.release.json"
        override_path.write_text(json.dumps(override, indent=2) + "\n", encoding="utf-8")
        bundle_dir = target / "release/bundle"
        before = artifact_snapshot(bundle_dir, bundles)
        run([pnpm, "--dir", "app", "exec", "tauri", "build", "--ci",
             "--config", str(override_path), "--bundles", ",".join(bundles), "--", "--locked"], env)
        bundle_env = dict(env)
        if source_state() != source:
            raise ValueError("release inputs changed during the build; rebuild from a stable tree")
        validate_built_artifacts(bundle_dir, bundles, before)
        # No Tauri target produces a portable build; it reuses the executable the
        # nsis build just compiled and is inspected alongside it.
        inspected = list(bundles)
        if "nsis" in bundles:
            build_portable_zip(target / "release/fontcarver-desktop.exe", scratch / "licenses",
                               version, bundle_dir / "portable")
            inspected.append("portable")
        run(budget, env)
        bundle_dir = target / "release/bundle"
        run([sys.executable, "scripts/check_release_bundle.py", "--bundle-dir", str(bundle_dir),
             "--expected", ",".join(inspected)], bundle_env)
        report_path = bundle_dir / "release-manifest.json"
        report = json.loads(report_path.read_text(encoding="utf-8"))
        report["source"] = source
        report["signing_verified"] = False
        report["publication_performed"] = False
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Verified FontCarver {version} native installers: {bundle_dir}")
        result = 0
    except (ValueError, VerificationError, OSError, subprocess.CalledProcessError) as error:
        print(f"Release build failed: {error}", file=sys.stderr)
    finally:
        # Scratch belongs to this entry point; retained installers/evidence live
        # in target/release/bundle. Never remove another task's Cargo artifacts.
        shutil.rmtree(scratch)
        try:
            scratch_root.rmdir()
        except OSError:
            pass  # Another build may own a separate temporary child.
        final_budget = subprocess.run(budget, cwd=ROOT, env=env, check=False)
        if final_budget.returncode:
            result = 1
    return result


if __name__ == "__main__":
    raise SystemExit(main())
