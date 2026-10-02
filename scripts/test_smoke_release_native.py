"""Regression checks for honest native installer startup evidence."""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from smoke_release_native import SmokeFailure, check_version, check_headless, check_startup, probe


class NativeSmokeTests(unittest.TestCase):
    def test_loader_failure_is_not_a_successful_headless_launch(self):
        with self.assertRaisesRegex(SmokeFailure, "no-display"):
            check_headless(127, "error while loading shared libraries: libgtk.so")

    def test_generic_crash_is_not_a_successful_headless_launch(self):
        for status in (0, 101, -11):
            with self.subTest(status=status), self.assertRaises(SmokeFailure):
                check_headless(status, "no graphical session is available")

    def test_expected_no_display_refusal_is_accepted(self):
        check_headless(1, "fontcarver-desktop: no graphical session is available (test)")

    def test_version_must_identify_this_exact_artifact(self):
        check_version(0, "fontcarver-desktop 0.1.0\n", "0.1.0")
        for status, text in ((1, "fontcarver-desktop 0.1.0"),
                             (0, "fontcarver-desktop 0.2.0"), (0, "")):
            with self.subTest(status=status, text=text), self.assertRaises(SmokeFailure):
                check_version(status, text, "0.1.0")

    def test_headless_hang_cannot_pass_even_with_the_expected_diagnostic(self):
        with self.assertRaisesRegex(SmokeFailure, "deadline"):
            check_startup({"exit_code": 1, "timed_out": True,
                           "output": "fontcarver-desktop: no graphical session is available"}, "headless")

    def test_gui_early_exit_is_rejected_even_when_successful(self):
        with self.assertRaisesRegex(SmokeFailure, "GUI process exited"):
            check_startup({"exit_code": 0, "timed_out": False, "output": ""}, "gui")
        check_startup({"exit_code": -15, "timed_out": True, "output": ""}, "gui")

    def test_timeout_kills_and_reaps_owned_process(self):
        result = probe([sys.executable, "-c", "import time; time.sleep(30)"],
                       dict(os.environ), seconds=0.05)
        self.assertTrue(result["timed_out"])
        self.assertIsNotNone(result["exit_code"])

    def test_output_is_collected_for_failure_evidence(self):
        result = probe([sys.executable, "-c", "print('loader failed'); raise SystemExit(7)"],
                       dict(os.environ), seconds=5)
        self.assertEqual(result["exit_code"], 7)
        self.assertFalse(result["timed_out"])
        self.assertEqual(result["output"].strip(), "loader failed")

    @unittest.skipIf(os.name == "nt", "POSIX process-group ownership check")
    def test_timed_out_process_cannot_leave_a_child_running(self):
        scratch = Path(__file__).resolve().parents[1] / ".scratch/p5f-w3-native"
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as temporary:
            marker = Path(temporary) / "child-survived"
            child = f"import time; from pathlib import Path; time.sleep(0.3); Path({str(marker)!r}).touch()"
            parent = f"import subprocess, sys, time; subprocess.Popen([sys.executable, '-c', {child!r}]); time.sleep(30)"
            result = probe([sys.executable, "-c", parent], dict(os.environ), seconds=0.1)
            subprocess.run([sys.executable, "-c", "import time; time.sleep(0.4)"], check=True)
            self.assertTrue(result["timed_out"])
            self.assertFalse(marker.exists())


if __name__ == "__main__":
    unittest.main()
