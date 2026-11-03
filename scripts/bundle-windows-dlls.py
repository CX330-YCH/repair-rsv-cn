#!/usr/bin/env python3
"""Stage only the helper's transitive PE imports, excluding Windows system DLLs."""

import argparse
from collections import deque
import os
from pathlib import Path
import re
import shutil
import subprocess


def imported_dlls(binary: Path, objdump: str) -> list[str]:
    result = subprocess.run([objdump, "-p", str(binary)], check=True, capture_output=True, text=True)
    return re.findall(r"^\s*DLL Name:\s*(\S+)\s*$", result.stdout, re.MULTILINE | re.IGNORECASE)


def collect_dependencies(helper: Path, dll_directory: Path, system_directory: Path, objdump: str) -> list[Path]:
    available = {path.name.casefold(): path for path in dll_directory.iterdir() if path.suffix.casefold() == ".dll"}
    system = {path.name.casefold() for path in system_directory.iterdir() if path.suffix.casefold() == ".dll"}
    selected = {}
    queue = deque([helper])
    while queue:
        binary = queue.popleft()
        for name in imported_dlls(binary, objdump):
            key = name.casefold()
            if key in selected:
                continue
            if key in available:
                selected[key] = available[key]
                queue.append(available[key])
            elif key not in system and not key.startswith(("api-ms-win-", "ext-ms-win-")):
                raise RuntimeError(f"Missing runtime dependency {name}, imported by {binary.name}")
    return [selected[key] for key in sorted(selected)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dll-dir", required=True, type=Path)
    parser.add_argument("--objdump", default="objdump")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    destination = root / "bin"
    helper = destination / "untrunc-rsv.exe"
    libraries = collect_dependencies(helper, args.dll_dir, Path(os.environ["SYSTEMROOT"]) / "System32", args.objdump)
    wanted = {library.name.casefold() for library in libraries}
    # bin/*.dll are generated runtime assets; remove leftovers from earlier all-DLL builds.
    for existing in destination.iterdir():
        if existing.suffix.casefold() == ".dll" and existing.name.casefold() not in wanted:
            existing.unlink()
    for library in libraries:
        shutil.copy2(library, destination / library.name)
    total = helper.stat().st_size + sum(library.stat().st_size for library in libraries)
    print(f"Staged {len(libraries)} required DLLs, {total / 1024 / 1024:.1f} MiB including the helper")
    for library in libraries:
        print(f"  {library.name}")


if __name__ == "__main__":
    main()
