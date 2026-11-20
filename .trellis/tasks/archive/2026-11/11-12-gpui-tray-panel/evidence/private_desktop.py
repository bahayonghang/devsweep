"""Run a bounded acceptance probe on its own non-input desktop."""

import ctypes
import subprocess
import sys
import uuid
from ctypes import wintypes
import win32api
import win32event
import win32process

args = sys.argv[1:]
user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.CreateDesktopW.argtypes = [
    wintypes.LPCWSTR,
    wintypes.LPCWSTR,
    wintypes.LPVOID,
    wintypes.DWORD,
    wintypes.DWORD,
    wintypes.LPVOID,
]
user32.CreateDesktopW.restype = wintypes.HANDLE
user32.CloseDesktop.argtypes = [wintypes.HANDLE]
name = "DevSweepReview_" + uuid.uuid4().hex
desktop = user32.CreateDesktopW(name, None, None, 0, 0x01FF, None)
if not desktop:
    raise ctypes.WinError(ctypes.get_last_error())
try:
    options = win32process.STARTUPINFO()
    options.lpDesktop = "winsta0\\" + name
    process, thread, pid, tid = win32process.CreateProcess(
        None,
        subprocess.list2cmdline(args),
        None,
        None,
        False,
        0x08000000,
        None,
        None,
        options,
    )
    thread.Close()
    try:
        if win32event.WaitForSingleObject(process, 120000) != 0:
            win32process.TerminateProcess(process, 2)
            raise TimeoutError("private desktop probe timed out")
        sys.exit(win32process.GetExitCodeProcess(process))
    finally:
        process.Close()
finally:
    user32.CloseDesktop(desktop)
