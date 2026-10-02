"""Owned Windows process-tree cleanup and fail-closed startup checks."""

import ctypes
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

from release_windows_job import ExtendedLimits, join_owned_job, main


class WindowsJobTests(unittest.TestCase):
    def kernel(self):
        return Mock(CreateJobObjectW=Mock(return_value=11),
                    SetInformationJobObject=Mock(return_value=True),
                    GetCurrentProcess=Mock(return_value=22),
                    AssignProcessToJobObject=Mock(return_value=True))

    def test_job_kills_all_members_when_its_last_handle_closes(self):
        kernel = self.kernel()
        def limits(_handle, _kind, pointer, _size):
            self.assertEqual(ctypes.cast(pointer, ctypes.POINTER(ExtendedLimits)).contents.basic.flags,
                             0x00002000)
            return True
        kernel.SetInformationJobObject.side_effect = limits
        self.assertEqual(join_owned_job(kernel), 11)
        kernel.AssignProcessToJobObject.assert_called_once_with(11, 22)
        kernel.CloseHandle.assert_not_called()

    def test_failed_job_assignment_closes_handle_and_aborts(self):
        kernel = self.kernel()
        kernel.AssignProcessToJobObject.return_value = False
        with self.assertRaisesRegex(OSError, "application was not launched"):
            join_owned_job(kernel)
        kernel.CloseHandle.assert_called_once_with(11)

    def test_unsafe_job_cannot_launch_the_application(self):
        with patch('release_windows_job.os.name', 'nt'), \
             patch('release_windows_job.join_owned_job', side_effect=OSError('job unavailable')), \
             patch('release_windows_job.subprocess.call') as launch:
            self.assertEqual(main(['application.exe']), 1)
            launch.assert_not_called()

    @unittest.skipUnless(os.name == "nt", "requires an actual Windows Job Object")
    def test_early_parent_exit_cannot_leave_an_orphan_child(self):
        scratch = Path(__file__).resolve().parents[1] / '.scratch/p5f-w3-native'
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as temporary:
            marker = Path(temporary) / 'orphan-survived'
            child = f"import time; from pathlib import Path; time.sleep(2); Path({str(marker)!r}).touch()"
            parent = f"import subprocess,sys; subprocess.Popen([sys.executable,'-c',{child!r}])"
            wrapper = Path(__file__).with_name('release_windows_job.py')
            result = subprocess.run([sys.executable, str(wrapper), sys.executable, '-c', parent],
                                    capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)
            time.sleep(2.5)
            self.assertFalse(marker.exists(), 'job left a child alive after its parent exited')


if __name__ == '__main__':
    unittest.main()
