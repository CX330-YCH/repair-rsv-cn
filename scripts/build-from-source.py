#!/usr/bin/env python3
"""Build the RSV helper using dependencies selected from the local environment.

The script deliberately keeps all build-specific overrides in environment variables so
macOS, Linux, and Windows can use the same entry point without modifying system paths.
"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "source" / "untrunc-rsv"
BIN = ROOT / "bin"


def command(name: str, *args: str, cwd: Path | None = None, env: dict[str, str] | None = None) -> str:
    result = subprocess.run(
        [name, *args],
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        if result.stdout:
            print(result.stdout, end="")
        if result.stderr:
            print(result.stderr, end="", file=sys.stderr)
        raise subprocess.CalledProcessError(result.returncode, [name, *args])
    return result.stdout.strip()


def find_tool(name: str, local_prefix: Path) -> str:
    configured = os.environ.get(name.upper().replace("-", "_"))
    if configured:
        return configured
    local = local_prefix / "bin" / name
    if local.is_file():
        return str(local)
    found = shutil.which(name)
    if found:
        return found
    raise SystemExit(f"Missing build tool: {name}. Set {name.upper()} or prepare .tooling first.")


def main() -> None:
    system = platform.system().lower()
    machine = platform.machine().lower()
    local_prefix = Path(
        os.environ.get("REPAIR_RSV_DEPS_PREFIX", str(ROOT / ".tooling" / "deps" / f"{system}-{machine}"))
    )
    local_prefix.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env.setdefault("PKG_CONFIG_ALLOW_SYSTEM_CFLAGS", "1")
    env.setdefault("PKG_CONFIG_ALLOW_SYSTEM_LIBS", "1")
    pkgconfig_dirs = [local_prefix / "lib" / "pkgconfig", local_prefix / "share" / "pkgconfig"]
    existing_pkgconfig = env.get("PKG_CONFIG_PATH", "")
    pkgconfig_values = [str(path) for path in pkgconfig_dirs if path.is_dir()]
    if existing_pkgconfig:
        pkgconfig_values.append(existing_pkgconfig)
    if pkgconfig_values:
        env["PKG_CONFIG_PATH"] = os.pathsep.join(pkgconfig_values)

    pkg_config = find_tool("pkg-config", local_prefix)
    make = find_tool("make", local_prefix)
    cxx = os.environ.get("CXX") or shutil.which("c++") or shutil.which("g++")
    if not cxx:
        raise SystemExit("Missing C++ compiler. Set CXX or prepare a compiler in .tooling.")

    env["CXX"] = cxx
    expected_ffmpeg = env.get("REPAIR_RSV_FFMPEG_VERSION", "9.0.2")
    ffmpeg = str(local_prefix / "bin" / "ffmpeg") if (local_prefix / "bin" / "ffmpeg").is_file() else shutil.which("ffmpeg")
    if expected_ffmpeg and ffmpeg:
        version_line = command(ffmpeg, "-version", env=env).splitlines()[0].split()
        actual_ffmpeg = version_line[2] if len(version_line) > 2 else ""
        if actual_ffmpeg != expected_ffmpeg:
            raise SystemExit(
                f"FFmpeg {expected_ffmpeg} is required, but {actual_ffmpeg or 'an unknown version'} was found"
            )
    cflags = command(pkg_config, "--cflags", "libavformat", "libavcodec", "libavutil", env=env)
    libs = command(pkg_config, "--libs", "libavformat", "libavcodec", "libavutil", env=env)
    jobs = os.environ.get("MAKE_JOBS", str(max(1, os.cpu_count() or 1)))
    output_name = "untrunc-rsv.exe" if os.name == "nt" else "untrunc-rsv"
    source_name = "untrunc.exe" if os.name == "nt" else "untrunc"

    command(make, "clean", cwd=SOURCE, env=env)
    command(
        make,
        f"-j{jobs}",
        f"FFMPEG_CFLAGS={cflags}",
        f"FFMPEG_LIBS={libs}",
        cwd=SOURCE,
        env=env,
    )

    built = SOURCE / source_name
    if not built.is_file():
        raise SystemExit(f"Build completed without producing {built}")
    BIN.mkdir(parents=True, exist_ok=True)
    destination = BIN / output_name
    shutil.copy2(built, destination)
    if os.name != "nt":
        destination.chmod(0o755)
    print(f"Updated {destination}")


if __name__ == "__main__":
    main()
