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

`bundle` 会根据当前系统选择 macOS DMG、Windows 单文件 EXE 或 Linux DEB/AppImage。构建产物位于 `.tooling/tauri-target/release/bundle/`；可以通过 `CARGO_TARGET_DIR` 指定其他工作区目录。macOS 的 dylib 收集和签名只在 macOS 分支执行。

Windows 构建前，将 `untrunc-rsv.exe` 和所有 FFmpeg/MinGW 运行时 DLL 放入仓库 `bin/`。Windows 的 `build.rs` 会把引擎、DLL 和许可证压缩嵌入主程序，不包含仓库中的 macOS helper。`tauri build --no-bundle` 生成 GUI 子系统程序，再复制到 `bundle/portable/Repair-RSV-<版本>-Windows-x64.exe`，不生成 MSI 或安装向导。

发布时只需下载并双击这一个 EXE，不需要旁置文件；首次运行会将内置引擎释放到 Windows 用户缓存 `com.repairrsv.desktop/engine/<内容哈希>/`，后续启动校验并复用，缓存损坏时自动重新释放。主程序、文件信息读取和修复子进程不会创建控制台窗口。界面仍使用 Windows 的 Microsoft Edge WebView2 Runtime，未安装该系统组件的机器需要先补装它。

GitHub Actions 只监听 `release: published`，构建 Apple Silicon、Windows x64 和 Linux x64；Intel Mac 暂停构建。Windows 产物上传前，会复制单个 EXE 到包含中文和空格的隔离目录，移除构建工具 DLL 搜索路径，检查 GUI PE 子系统、应用启动和释放后的引擎 `--probe`；如果任一检查失败，不上传该发布的附件。
