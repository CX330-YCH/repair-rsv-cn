# Workspace-local build environment

This directory is intentionally kept outside version control except for this note. Use:

```sh
source scripts/local-env.sh
```

The shell setup redirects Rustup, Cargo, npm's cache, and the FFmpeg dependency prefix into `.tooling/`. It does not change system-wide configuration.

The cross-platform helper build expects `pkg-config`, FFmpeg development libraries (`libavformat`, `libavcodec`, and `libavutil`), a C++ compiler, and `make` to be available either in `.tooling/bin` or on the current `PATH`. Set `REPAIR_RSV_DEPS_PREFIX` when those libraries are staged elsewhere inside the workspace.

The repository currently expects FFmpeg `9.0.2` (`REPAIR_RSV_FFMPEG_VERSION`). The helper checks that version when an `ffmpeg` executable is available, so a newer system installation does not silently change a release build.

For a local Rust installation, install Rustup with the environment above already sourced and use `--no-modify-path`. The installer will then keep its state under `.tooling/rustup` and `.tooling/cargo`.
