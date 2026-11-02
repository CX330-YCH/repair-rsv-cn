# 跨平台本地构建

本项目的桌面端使用 Tauri，恢复引擎使用 C++ 和 FFmpeg。构建缓存、Rustup、Cargo、npm 缓存和本地依赖前缀统一放在仓库的 `.tooling/`，不会写入系统级配置。

先在仓库根目录执行：

```sh
source scripts/local-env.sh
npm --prefix app ci
```

工作区当前固定的发布构建组件为：

- Rust `1.99.1`，见 [rust-toolchain.toml](../rust-toolchain.toml)
- Tauri `2.10.3`、Tauri Build `2.5.6`、Dialog 插件 `2.7.0`
- Tauri JavaScript API / CLI `2.10.1`
- TypeScript `5.9.3`、Vite `6.4.2`
- FFmpeg `9.0.2`

恢复引擎构建从 `REPAIR_RSV_DEPS_PREFIX` 读取 `pkg-config` 和 FFmpeg 开发库。默认目录是 `.tooling/deps/<系统>-<架构>`，也可以指向工作区内的其他依赖目录：

```sh
export REPAIR_RSV_DEPS_PREFIX="$PWD/.tooling/deps/darwin-arm64"
python3 scripts/build-from-source.py
```

验证和打包入口已经不依赖 macOS 专用的 zsh：

```sh
npm --prefix app run check:tauri
npm --prefix app run web:build
npm --prefix app run bundle
```

`bundle` 会根据当前系统选择 DMG、NSIS/MSI 或 DEB/AppImage；macOS 的 dylib 收集和签名仍只在 macOS 分支执行。跨平台 GitHub Actions 发布矩阵尚未加入本次改造。
