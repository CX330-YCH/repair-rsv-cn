<p align="center">
  <img src="app/src-tauri/icons/icon.png" alt="Repair RSV 图标" width="112" height="112">
</p>

<h1 align="center">Repair RSV 中文改进版</h1>

<p align="center">用于 macOS 的 Sony RSV 视频修复工具，支持中文界面、文件信息显示及原始视频流恢复。</p>

本项目基于 [macmixing/repair-rsv](https://github.com/macmixing/repair-rsv) 修改，重点修复部分 Sony RSV 文件恢复后的周期性卡顿、声音断续和音频索引异常，并完善桌面 App。恢复引擎来自 [anthwlock/untrunc](https://github.com/anthwlock/untrunc)，原始项目为 [ponchio/untrunc](https://github.com/ponchio/untrunc)。感谢各上游项目的作者与贡献者。

这是独立维护的改进版本；原项目主页、版权声明及许可证均予以保留。详见 [项目来源与许可说明](NOTICE.md)。

## 下载与使用

在 [Releases](https://github.com/CX330-YCH/repair-rsv-cn/releases) 下载最新安装包。当前提供 **macOS Apple Silicon** 的 DMG 和 ZIP；安装包使用本地 ad-hoc 签名，尚未进行 Apple 公证。

1. 打开 Repair RSV，选择或拖入损坏的 `.RSV` 文件。
2. 选择同一相机、同一录制模式的正常视频作为参照，尤其注意编码、分辨率、帧率和音频设置。
3. 点击「开始修复」。完成后点击「显示输出文件」。

输出保存在 RSV 同目录，名称为：

```text
原文件名.RSV_fixed-rsv.MP4
```

输出 MP4 保留可恢复的原始视频流和 PCM 音频，不进行重编码。未完成写入的视频帧不会补造，找不到的尾部音频也不会补音。请保留原始文件，使用副本进行恢复和回放验证。

## 主要功能

- **音视频边界校验**：检查 NAL 头、类型和长度，结合多个完整分组、视频时长及 PCM 采样对齐确认音频边界，避免把音频当作视频。
- **截断恢复**：保留尾部已确认完整的视频帧，排除截断样本，不再从文件末尾直接倒扣固定音频块。
- **索引修正**：按实际 NAL 类型识别关键帧，并将显示时间表的样本覆盖数限制到实际恢复数量。
- **输入文件信息**：在两个文件选择区域下方显示实际可读的参数，包括分辨率、帧率、视频时长、大小、视频编码、位深和音频信息。原始 RSV 只显示能够确认的字段。
- **结果信息**：在进度条下方显示修复文件的参数、视频帧数和音频时长。
- **中英文界面**：首次启动跟随系统语言，可手动切换并记住选择；菜单及原生文件选择器支持中文。
- **简洁状态显示**：去除占位说明，保留实际参数、进度和必要的错误提示；详细运行日志按需展开。

完整更新记录见 [CHANGELOG.md](CHANGELOG.md)。

## 已验证的恢复结果

在一份 H.264 High 4:2:2、10 bit、1920×1080、50 fps，搭配 48 kHz / 16 bit / 双声道 PCM 的真实 RSV 样本上：

| 项目 | 原修复结果 | 本版恢复结果 |
| --- | --- | --- |
| 实际可解码视频帧 | 2,980 | 3,108 |
| 相邻画面显示间隔 | 128 处为 40 ms | 全部为 20 ms |
| 音频轨时长 | 约 18.02 秒 | 61.92 秒 |
| 视频轨时长 | 62.18 秒，含截断样本 | 62.16 秒，保留完整样本 |

新版结果已通过全片严格音视频解码、时间戳检查和人工声音、口型回放确认。App 输出与独立索引校正结果逐字节一致。

以上是该样本的实测结果，不代表所有损坏文件都能达到同样恢复程度。HEVC 已加入解析器合成数据测试，其他录制模式仍需更多真实样本验证。测试媒体未随仓库或 Release 发布。

## 源码结构

| 路径 | 内容 |
| --- | --- |
| `app/` | Tauri 桌面 App，包含 TypeScript 前端、中文翻译及 Rust 后端 |
| `source/untrunc-rsv/` | 完整恢复引擎源码，保留上游说明与许可证 |
| `source/untrunc-rsv/src/rsv_parser.h` | 有边界检查的 AVC / HEVC NAL 解析器 |
| `source/untrunc-rsv/src/media_info.cpp` | 基于 FFmpeg 库的媒体信息读取 |
| `scripts/` | 引擎编译、动态库打包、签名、公证和测试脚本 |
| `tests/` | 解析器、恢复流程和媒体信息回归测试 |
| `docs/releases/` | 中文版本发布说明 |
| `LICENSE` | 项目沿用的 GPL 第 3 版许可证全文 |

仓库以当前版本源码快照发布。生成的 `lib/`、构建缓存和本地媒体不进入版本控制；安装包在 Release 中提供。

## 从源码构建

需要 macOS、Xcode Command Line Tools、Homebrew、Node.js、Rust stable 和 Python 3。首次构建需要联网下载 npm 与 Cargo 依赖。

```zsh
xcode-select --install
brew install ffmpeg pkgconf node rust python

git clone https://github.com/CX330-YCH/repair-rsv-cn.git
cd repair-rsv-cn
npm --prefix app ci

# 编译恢复引擎
zsh scripts/build-from-source.sh

# 运行桌面 App 开发版
cd app
npm run dev
```

本次发布构建使用 FFmpeg 9.0.2。App 通过随包引擎读取媒体信息，不依赖用户另外安装 FFprobe。

### 打包 App

```zsh
cd app
npm run bundle
```

打包流程会重新编译恢复引擎、收集 FFmpeg / Homebrew 动态库、改写包内依赖路径并签名。默认使用本地 ad-hoc 签名，不会自动完成 Apple 公证。产物位于：

```text
app/src-tauri/target/release/bundle/
```

如需用自己的 Developer ID 分发：

```zsh
SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)" npm run bundle
```

配置自己的 Apple 公证凭据后，可执行 `npm run notarize`。相关脚本不会要求使用原作者的证书。

## 命令行使用

在仓库根目录编译引擎后运行：

```zsh
./bin/untrunc-rsv -rsv "/path/to/参照视频.MP4" "/path/to/损坏文件.RSV"
```

也可以通过包装脚本运行：

```zsh
zsh scripts/recover-rsv.sh "/path/to/参照视频.MP4" "/path/to/损坏文件.RSV"
```

指定输出路径，或只读取文件信息：

```zsh
./bin/untrunc-rsv -rsv -dst "/path/to/输出.MP4" "/path/to/参照视频.MP4" "/path/to/损坏文件.RSV"
./bin/untrunc-rsv --probe "/path/to/视频.MP4"
```

`--probe` 返回 JSON，不修改输入文件。仓库内已有的 helper 在打包后使用包内动态库路径；单独使用时请先运行引擎编译脚本。

## 测试

```zsh
# NAL 解析器回归，启用 AddressSanitizer / UndefinedBehaviorSanitizer
zsh scripts/test-rsv.sh

# 使用本地媒体进行恢复集成测试，需要 FFmpeg / FFprobe
zsh scripts/test-rsv.sh "/path/to/参照视频.MP4" "/path/to/损坏文件.RSV"

# 将引擎读取的媒体信息与 FFprobe 对比
python3 tests/media_info_test.py "/path/to/损坏文件.RSV" "/path/to/参照视频.MP4" "/path/to/修复视频.MP4"

# 前端类型检查及生产构建
npm --prefix app run web:build
```

恢复集成测试使用临时副本，要求输入符合每个元数据包对应一个视频帧、分组节奏固定、每组音频采样数为整数的 Sony AVC / 16 bit PCM 布局，且至少含四个完整组。测试不会修改或上传提供的媒体。

## 项目来源与许可证

- **直接上游**：[macmixing/repair-rsv](https://github.com/macmixing/repair-rsv)，原作者 Dom Esposito；本项目保留其源码、版权信息及 GPL-3.0-or-later 授权声明。
- **恢复引擎上游**：[anthwlock/untrunc](https://github.com/anthwlock/untrunc)；原始项目为 [ponchio/untrunc](https://github.com/ponchio/untrunc)。引擎的 GPL-2.0-or-later 声明及 [COPYING](source/untrunc-rsv/COPYING) 原文保留。
- **本项目整体发行**：沿用 **GPL-3.0-or-later**，许可证全文见 [LICENSE](LICENSE)。
- **FFmpeg 与其他依赖**：见 [NOTICE.md](NOTICE.md)，依赖各自的版权及许可证继续适用。

本工具按现状提供，不保证恢复成功。发布包包含完整源码入口和构建脚本，欢迎提交可复现的问题及改进建议。
