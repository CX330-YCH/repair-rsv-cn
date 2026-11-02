# 项目来源与许可说明

本文件记录本项目的来源和上游主页，不替代各许可证原文。

## Repair RSV

- 直接上游：https://github.com/macmixing/repair-rsv
- 原作者：Dom Esposito（macmixing）。
- 本项目基于该项目的 `2e82b08` 提交继续修改，保留其源码中的版权声明。
- 上游声明使用 GPL-3.0-or-later；许可证全文保留在根目录 `LICENSE`。
- 本改进版维护与发布：CX330-YCH，https://github.com/CX330-YCH/repair-rsv-cn 。

本版本的修复与新增功能由本改进版维护，不代表原作者对这些改动的认可或担保。

## untrunc 恢复引擎

- 改进版上游：https://github.com/anthwlock/untrunc
- 原始项目：https://github.com/ponchio/untrunc
- 原始源码包含 Federico Ponchio 等作者的版权声明，相关声明保持原样。
- 源码声明使用 GPL-2.0-or-later。原许可证全文为 `source/untrunc-rsv/COPYING`，上游说明为 `source/untrunc-rsv/README.md`。
- 本项目整体发行沿用 GPL-3.0-or-later。

## FFmpeg 及构建依赖

- FFmpeg 主页：https://ffmpeg.org/
- 本次发布使用的 FFmpeg 版本：9.0.2；本机构建的 FFmpeg 声明为 GPL-3.0-or-later。
- 对应版本源码：https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz
- Homebrew FFmpeg 构建配方：https://github.com/Homebrew/homebrew-core/blob/HEAD/Formula/f/ffmpeg.rb
- FFmpeg 上游许可说明：https://ffmpeg.org/legal.html
- Tauri：https://github.com/tauri-apps/tauri ，使用其上游 MIT / Apache-2.0 授权。
- 前端与 Rust 依赖的具体版本保留在 `app/package-lock.json` 和 `app/src-tauri/Cargo.lock`，各依赖的版权与许可证按其上游声明适用。

本项目保留上游许可证，不以自定义许可证替换。Release 同时提供源码归档和许可证文件。
