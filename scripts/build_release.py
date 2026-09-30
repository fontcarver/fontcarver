#!/usr/bin/env python3
"""Build and inspect native installers without publishing or system installation.

Requires Python 3.11+, pinned Rust/pnpm, frontend dependencies and Tauri's native
system prerequisites. Use the same Cargo target directory as development.
"""

from __future__ import annotations

import argparse
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

ROOT = Path(__file__).resolve().parents[1]


def native_bundles(system: str) -> list[str]:
    """Return only the installer formats maintained for a native platform."""
    bundles = {"Linux": ["deb", "appimage"], "Darwin": ["dmg"], "Windows": ["msi"]}
    if system not in bundles:
        raise ValueError(f"unsupported release build platform: {system}")
    return bundles[system]


def validate_versions(cargo: str, frontend: str, tauri: str) -> str:
    """Reject mismatched package versions and versions Windows MSI cannot encode."""
    if len({cargo, frontend, tauri}) != 1:
        raise ValueError(f"version mismatch: Cargo={cargo}, frontend={frontend}, Tauri={tauri}")
    if not re.fullmatch(r"\d+\.\d+\.\d+", cargo):
        raise ValueError("release version must be a numeric major.minor.patch accepted by MSI")
    if any(int(value) > limit for value, limit in zip(cargo.split("."), [255, 255, 65535])):
        raise ValueError("release version exceeds Windows MSI version limits")
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
        bundles = args.bundles.split(",") if args.bundles else native
        if not bundles or any(bundle not in native for bundle in bundles):
            raise ValueError(f"native bundle formats on this platform are: {', '.join(native)}")
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        frontend = json.loads((ROOT / "app/package.json").read_text(encoding="utf-8"))
        config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
        version = validate_versions(manifest["workspace"]["package"]["version"],
                                    frontend["version"], config["version"])
        pnpm = shutil.which("pnpm")
        if pnpm is None:
            raise ValueError("pnpm is not installed; use the version pinned in package.json")
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
        run([pnpm, "--dir", "app", "exec", "tauri", "build", "--ci",
             "--config", str(override_path), "--bundles", ",".join(bundles), "--", "--locked"], env)
        run(budget, env)
        bundle_dir = target / "release/bundle"
        run([sys.executable, "scripts/check_release_bundle.py", "--bundle-dir", str(bundle_dir),
             "--expected", ",".join(bundles)], env)
        print(f"Verified FontCarver {version} native installers: {bundle_dir}")
        result = 0
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
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
