#!/usr/bin/env python3
"""Build the desktop bundle with workspace-local tool and dependency paths."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "app"
TOOLING = ROOT / ".tooling"


def run(*args: str, env: dict[str, str]) -> None:
    subprocess.run(args, cwd=ROOT, env=env, check=True)


def main() -> None:
    system = platform.system().lower()
    env = os.environ.copy()
    env.setdefault("CARGO_HOME", str(TOOLING / "cargo"))
    env.setdefault("RUSTUP_HOME", str(TOOLING / "rustup"))
    env.setdefault("NPM_CONFIG_CACHE", str(TOOLING / "npm-cache"))
    env.setdefault("REPAIR_RSV_DEPS_PREFIX", str(TOOLING / "deps" / f"{system}-{platform.machine().lower()}"))
    env["PATH"] = os.pathsep.join([str(TOOLING / "bin"), str(Path(env["CARGO_HOME"]) / "bin"), env.get("PATH", "")])
    npm_name = "npm.cmd" if os.name == "nt" else "npm"
    npm = shutil.which(npm_name) or npm_name

    (ROOT / "lib").mkdir(parents=True, exist_ok=True)
    if env.get("REPAIR_RSV_SKIP_HELPER_BUILD") != "1":
        run(sys.executable, str(ROOT / "scripts" / "build-from-source.py"), env=env)
    run(npm, "--prefix", str(APP), "run", "web:build", env=env)

    if system == "darwin":
        run("zsh", str(ROOT / "scripts" / "bundle-ffmpeg-dylibs.sh"), env=env)
        run("zsh", str(ROOT / "scripts" / "sign-runtime-assets.sh"), env=env)
        bundles = "dmg"
    elif system == "windows":
        bundles = "nsis,msi"
    else:
        bundles = "deb,appimage"

    run(npm, "--prefix", str(APP), "exec", "--", "tauri", "build", "--bundles", bundles, env=env)


if __name__ == "__main__":
    main()
