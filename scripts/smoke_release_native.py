#!/usr/bin/env python3
"""Check an installed native executable without sending desktop input.

The GUI check proves only that the process survives startup, not that its
frontend rendered or that interactive platform parity has been verified.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


class SmokeFailure(ValueError):
    """The installed executable failed a native smoke requirement."""


def check_version(status: int, output: str, version: str) -> None:
    """Require the installed binary to identify the exact expected release."""
    if status != 0 or output.strip() != f"fontcarver-desktop {version}":
        raise SmokeFailure(f"version probe failed (exit {status}): {output.strip()}")


def check_headless(status: int, output: str) -> None:
    """Accept only the shell's deliberate no-display error, never loader crashes."""
    if status != 1 or "fontcarver-desktop: no graphical session is available" not in output:
        raise SmokeFailure(f"expected deliberate no-display refusal (exit {status}): {output.strip()}")


def check_startup(result: dict, mode: str) -> None:
    """Distinguish a prompt refusal from a GUI process surviving observation."""
    if mode == "headless":
        if result["timed_out"]:
            raise SmokeFailure("headless process did not exit before its deadline")
        check_headless(result["exit_code"], result["output"])
    elif not result["timed_out"]:
        raise SmokeFailure("GUI process exited before the startup observation interval")


def stop_owned(process: subprocess.Popen) -> None:
    """Stop only the launched process and its children, then reap it."""
    if os.name == "nt":
        if process.poll() is None:
            # Terminating our wrapper closes its job handle. The job also owns
            # descendants whose immediate parent already exited.
            process.kill()
    else:
        # The group is created by probe; never target the caller's group.
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)
    if os.name != "nt":
        # A child can outlive its parent or ignore TERM. Its group still belongs
        # to this probe even after the leader has exited.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def probe(command: list[str], env: dict[str, str], seconds: float) -> dict:
    """Capture a bounded process run and always dispose of its owned children."""
    if os.name == "nt":
        # The wrapper joins an owned kill-on-close job before spawning the app.
        # Its exit kills descendants even when the app exited and orphaned them.
        command = [sys.executable, str(Path(__file__).with_name("release_windows_job.py")), *command]
    start = time.monotonic()
    with tempfile.TemporaryFile() as output:
        process = subprocess.Popen(command, env=env, stdout=output,
                                   stderr=subprocess.STDOUT, start_new_session=os.name != "nt")
        timed_out = False
        try:
            try:
                process.wait(timeout=seconds)
            except subprocess.TimeoutExpired:
                timed_out = True
        finally:
            stop_owned(process)
        output.seek(0)
        # Error evidence should remain small even if a broken runtime logs in a loop.
        text = output.read(64 * 1024).decode("utf-8", errors="replace")
    return {"exit_code": process.returncode, "timed_out": timed_out,
            "seconds": round(time.monotonic() - start, 3), "output": text}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--launch", choices=("headless", "gui"), required=True)
    parser.add_argument("--version", help="expected version; defaults to tauri.conf.json")
    parser.add_argument("--seconds", type=float, default=10,
                        help="startup survival interval for disposable native GUI runners")
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    report = {"platform": platform.system(), "executable": str(args.executable.resolve()),
              "launch_mode": args.launch, "ok": False, "signing_verified": False,
              "interactive_parity_verified": False}
    scratch = ROOT / ".scratch/p5-release-native"
    scratch.mkdir(parents=True, exist_ok=True)
    try:
        if not args.executable.is_file():
            raise SmokeFailure(f"installed executable missing: {args.executable}")
        if not 0 < args.seconds <= 60:
            raise SmokeFailure("--seconds must be greater than zero and at most 60")
        if args.launch == "headless" and platform.system() != "Linux":
            raise SmokeFailure("the headless no-display check applies to Linux only")
        version = args.version or json.loads(
            (ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))["version"]
        with tempfile.TemporaryDirectory(prefix="probe-", dir=scratch) as temporary:
            env = dict(os.environ)
            env.update(FC_CONFIG_DIR=temporary, TMPDIR=temporary, TMP=temporary,
                       TEMP=temporary, APPIMAGE_EXTRACT_AND_RUN="1")
            if args.launch == "headless":
                env.pop("WAYLAND_DISPLAY", None)
                env.update(GDK_BACKEND="x11", DISPLAY=":65535", XDG_RUNTIME_DIR=temporary)
            binary = str(args.executable.resolve())
            report["version"] = probe([binary, "--version"], env, seconds=30)
            if report["version"]["timed_out"]:
                raise SmokeFailure("version probe did not exit before its deadline")
            check_version(report["version"]["exit_code"], report["version"]["output"], version)
            report["startup"] = probe([binary], env, seconds=args.seconds)
            check_startup(report["startup"], args.launch)
            report["ok"] = True
            report["scope"] = ("native loader and expected no-display refusal" if args.launch == "headless"
                               else "native loader and bounded GUI process survival; UI readiness unverified")
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report["error"] = str(error)
        print(f"FAIL: {error}", file=sys.stderr)
    finally:
        try:
            scratch.rmdir()
        except OSError:
            pass
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["ok"]:
        print(f"PASS: {report['scope']}; {args.report}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
