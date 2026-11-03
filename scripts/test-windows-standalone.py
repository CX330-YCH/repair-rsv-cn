#!/usr/bin/env python3
"""Smoke-test the release EXE without any sibling resources or build-time DLL PATH."""

import json
import ctypes
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import time


def window_visible(process_id):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    found = []
    callback_type = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_void_p, ctypes.c_ssize_t)
    user32.IsWindowVisible.argtypes = [ctypes.c_void_p]
    user32.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
    user32.EnumWindows.argtypes = [callback_type, ctypes.c_ssize_t]
    @callback_type
    def visit(window, _):
        pid = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(window, ctypes.byref(pid))
        if pid.value == process_id and user32.IsWindowVisible(window):
            found.append(window)
        return 1
    user32.EnumWindows(visit, 0)
    return bool(found)


def main():
    executable = Path(sys.argv[1]).resolve()
    with executable.open("rb") as file:
        assert file.read(2) == b"MZ", "Not a Windows executable"
        file.seek(0x3C)
        pe_offset = struct.unpack("<I", file.read(4))[0]
        file.seek(pe_offset)
        assert file.read(4) == b"PE\0\0", "Missing PE header"
        file.seek(pe_offset + 24 + 68)
        assert struct.unpack("<H", file.read(2))[0] == 2, "App still uses the console subsystem"

    with tempfile.TemporaryDirectory(prefix="rsv-standalone-") as temporary:
        root = Path(temporary)
        isolated = root / "中文路径 with spaces"
        isolated.mkdir()
        standalone = isolated / executable.name
        shutil.copy2(executable, standalone)
        assert len(list(isolated.iterdir())) == 1, "Only the EXE should be distributed"
        env = os.environ.copy()
        system_root = Path(os.environ["SYSTEMROOT"])
        # No MSYS2 DLLs or repository paths available to accidentally mask omissions.
        env["PATH"] = os.pathsep.join([str(system_root / "System32"), str(system_root)])
        engine_cache = Path(env["LOCALAPPDATA"]) / "com.repairrsv.desktop" / "engine"
        env.pop("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", None)
        started = time.monotonic()
        app = subprocess.Popen([str(standalone)], cwd=isolated, env=env)
        try:
            deadline = time.monotonic() + 120
            helper = None
            visible_at = None
            while time.monotonic() < deadline:
                if app.poll() is not None:
                    raise RuntimeError(f"Standalone app exited early: {app.returncode}")
                if visible_at is None and window_visible(app.pid):
                    visible_at = time.monotonic()
                candidates = [path for path in engine_cache.glob("*/untrunc-rsv.exe") if "-staging-" not in str(path)]
                if candidates and visible_at is not None:
                    helper = candidates[0]
                    break
                time.sleep(0.5)
            assert helper is not None, "App did not extract the embedded engine"
            print(f"Window visible after {visible_at - started:.2f}s; engine cache ready after {time.monotonic() - started:.2f}s")
            fixture = root / "probe.mp4"
            fixture.write_bytes(b"probe fixture" * 16)
            probe = subprocess.run(
                [str(helper), "--probe", str(fixture)], cwd=isolated, env=env,
                check=True, capture_output=True, text=True, timeout=30,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            info = json.loads(probe.stdout)
            assert info["sizeBytes"] == fixture.stat().st_size
            # Allow setup/menus to finish, then confirm the UI process remains healthy.
            time.sleep(2)
            assert app.poll() is None, "App failed after extracting its engine"
            print("Windows GUI EXE and embedded engine passed isolated startup/probe checks")
        finally:
            # Stop this app's WebView children before removing the temporary cache.
            subprocess.run([str(system_root / "System32/taskkill.exe"), "/PID", str(app.pid), "/T", "/F"], capture_output=True)
            app.wait(timeout=30)


if __name__ == "__main__":
    main()
