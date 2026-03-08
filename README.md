# FFmpeg 工具插件 (FFmpeg Utils)

这是 **Ting Reader** 的一个基础工具插件，旨在为其他插件提供 FFmpeg 二进制文件支持。它作为一个中心化的依赖提供者，避免了每个插件（如 `m4a-format`）都需要重复内置 FFmpeg 可执行文件的问题。

## 功能特性

- **中心化 FFmpeg 提供者**：其他插件可以自动检测并使用此插件提供的 FFmpeg 二进制文件。
- **跨平台支持**：支持 Windows、Linux。
- **版本检查**：包含基础的版本验证功能。

## 安装说明

1.  下载对应您操作系统的最新发行版。
2.  将 `ffmpeg-utils` 文件夹解压到您的 Ting Reader `plugins` 目录下。
    - 目录结构应为：`plugins/ffmpeg-utils/`
3.  重启 Ting Reader。

## 开发者指南

如果您正在开发一个需要使用 FFmpeg 的插件，可以按照以下逻辑查找此插件提供的二进制文件。

**路径解析逻辑：**
1.  检查 `../ffmpeg-utils/bin/ffmpeg`（相对于您的插件目录）。
2.  检查 `../ffmpeg-utils/ffmpeg`（相对于您的插件目录）。

## 源码构建

### 前置要求
- Rust (最新稳定版)
- Cargo

### 构建命令
```bash
cargo build --release
```
编译后的库文件 (`ffmpeg_utils.dll` / `libffmpeg_utils.so` / `libffmpeg_utils.dylib`) 将位于 `target/release/` 目录中。

## 许可证

MIT License. 详见 [LICENSE](LICENSE) 文件。
