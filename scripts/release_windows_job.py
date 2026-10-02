#!/usr/bin/env python3
"""Run one native smoke process inside an owned Windows kill-on-close job.

The wrapper joins the job before launching the application. Descendants inherit
membership even if their immediate parent exits. Its last job handle closes
when this wrapper exits or is terminated, disposing of the whole owned tree.
"""

from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
import subprocess
import sys


class BasicLimits(ctypes.Structure):
    _fields_ = [("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64),
                ("flags", ctypes.c_uint32), ("minimum_working_set", ctypes.c_size_t),
                ("maximum_working_set", ctypes.c_size_t), ("active_processes", ctypes.c_uint32),
                ("affinity", ctypes.c_size_t), ("priority", ctypes.c_uint32),
                ("scheduling", ctypes.c_uint32)]


class IoCounters(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in
               ("reads", "writes", "other", "read_bytes", "write_bytes", "other_bytes")]


class ExtendedLimits(ctypes.Structure):
    _fields_ = [("basic", BasicLimits), ("io", IoCounters),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t)]


def join_owned_job(kernel=None):
    """Join a fresh job before spawning anything; a failed assignment aborts."""
    kernel = kernel or ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
    kernel.CreateJobObjectW.restype = wintypes.HANDLE
    kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int,
                                              ctypes.c_void_p, ctypes.c_uint32]
    kernel.SetInformationJobObject.restype = wintypes.BOOL
    kernel.GetCurrentProcess.argtypes = []
    kernel.GetCurrentProcess.restype = wintypes.HANDLE
    kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel.AssignProcessToJobObject.restype = wintypes.BOOL
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL
    handle = kernel.CreateJobObjectW(None, None)
    if not handle:
        raise OSError("could not create the native smoke process job")
    limits = ExtendedLimits()
    limits.basic.flags = 0x00002000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    if not kernel.SetInformationJobObject(handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
        kernel.CloseHandle(handle)
        raise OSError("could not set kill-on-close for the native smoke process job")
    if not kernel.AssignProcessToJobObject(handle, kernel.GetCurrentProcess()):
        kernel.CloseHandle(handle)
        raise OSError("could not join the native smoke process job; application was not launched")
    # Deliberately keep this non-inheritable handle until this wrapper exits.
    # Closing it here would terminate the wrapper as well as its descendants.
    return handle


def main(command: list[str]) -> int:
    if os.name != "nt" or not command:
        print("Windows job wrapper requires Windows and an executable", file=sys.stderr)
        return 2
    try:
        # The OS owns this handle's lifetime until wrapper process exit. It is
        # deliberately not passed to CloseHandle while this member is running.
        _job = join_owned_job()
        return subprocess.call(command)
    except OSError as error:
        print(f"native smoke job failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
