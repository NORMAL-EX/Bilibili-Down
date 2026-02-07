# Bilibili-Down

[English](./README_en.md) | 简体中文

一个现代化的哔哩哔哩视频下载工具，使用 Rust 开发，基于 egui 图形界面框架构建。

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)

## 👉 官方网站

https://bilibili-down.github.io

## 🌟 特性

- 🎥 **高质量视频下载** - 支持最高 8K 超高清下载
- ❌ **免登录高质量视频下载** - 破甲 B站 wbi 鉴权，实现最高 1080P 分辨率的视频免登录下载
- 🎵 **音频提取** - 支持下载 MP3 格式音频
- 🔐 **登录支持** - 二维码登录获取高清视频权限
- ⚡ **多线程下载** - 基于 aria2c 的高速多线程下载
- 🎨 **现代化界面** - 直观易用的图形界面
- 🌐 **多语言支持** - 中文/英文界面切换
- 🎯 **智能解析** - 支持 BV 号和视频链接解析
- 📊 **下载管理** - 实时进度显示和队列管理
- 🔄 **自动合并** - 视频音频智能合并

## 🚀 快速开始

### 系统要求

- Windows / macOS / Linux
- 网络连接

### 安装步骤

1. **下载发行版**
   ```
   从 Releases 页面下载最新版本
   ```

2. **解压文件**
   ```
   将压缩包解压到任意目录
   ```

3. **运行程序**
   ```
   双击 bilibili-down 启动程序
   ```

### 目录结构

```
bilibili-down/
├── bilibili-down    # 主程序
├── config.json      # 配置文件（首次运行自动生成）
└── tools/
   ├── aria2c       # 下载工具（已包含）
   └── ffmpeg       # 视频处理工具（已包含）
```

## 📖 使用说明

### 基础使用

1. **启动程序** - 双击 `bilibili-down`

2. **输入视频信息**
   - 支持 BV 号：`BV1xx411c7mD`
   - 支持完整链接：`https://www.bilibili.com/video/BV1xx411c7mD`
   - 支持短链接：`https://b23.tv/xxxxx`

3. **解析视频** - 点击「解析」按钮获取视频信息

4. **选择质量** - 在弹出窗口中选择所需的视频质量

5. **开始下载** - 选择下载视频或 MP3 音频

### 高级功能

#### 登录账号
- 点击右上角头像图标
- 使用手机扫描二维码登录
- 登录后可下载高清视频（1080P+）

#### 下载管理
- **下载队列** - 查看所有下载任务
- **暂停/继续** - 控制下载进度
- **删除任务** - 移除不需要的下载

#### 设置选项
- **主题设置** - 系统/明亮/暗黑主题
- **语言设置** - 中文/英文界面
- **下载路径** - 自定义下载目录
- **线程数量** - 调整下载线程数

### 支持的视频质量

| 质量 | 描述 | 登录要求 |
|------|------|----------|
| 8K | 超高清 | ✅ |
| 4K | 超清 | ✅ |
| 1080P 60帧 | 高帧率 | ✅ |
| 1080P+ | 高码率 | ✅ |
| 1080P | 高清 | ❌ |
| 720P | 高清 | ❌ |
| 480P | 清晰 | ❌ |
| 360P | 流畅 | ❌ |

## 🛠️ 开发

### 构建环境

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/NORMAL-EX/bilibili-down.git
cd bilibili-down

# 构建项目
cargo build --release

# 运行项目
cargo run
```

### 项目架构

```
src/
├── main.rs              # 程序入口点
├── app.rs               # 主应用逻辑
├── config.rs            # 配置管理
├── bilibili.rs          # B站API接口
├── downloader.rs        # 下载管理器
└── ui/
    ├── mod.rs           # UI模块定义
    ├── home.rs          # 首页界面
    ├── download_queue.rs # 下载队列界面
    ├── settings.rs      # 设置界面
    ├── login.rs         # 登录窗口
    └── video_detail.rs  # 视频详情窗口
```

### 技术栈

- **GUI框架**: [egui](https://github.com/emilk/egui) - 即时模式GUI
- **HTTP客户端**: [reqwest](https://github.com/seanmonstar/reqwest) - 异步HTTP库
- **异步运行时**: [tokio](https://tokio.rs/) - 异步运行时
- **序列化**: [serde](https://serde.rs/) - JSON序列化
- **下载引擎**: [aria2](https://aria2.github.io/) - 多线程下载
- **视频处理**: [FFmpeg](https://ffmpeg.org/) - 音视频处理

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

### 开发规范

- 遵循 Rust 代码风格指南
- 添加必要的注释和文档
- 确保所有测试通过
- 更新相关文档

## 📝 免责声明

**重要提醒**：

1. 🆓 **本软件完全免费** - 请谨防上当受骗
2. 📚 **仅供学习研究** - 请勿用于商业用途
3. 🎯 **尊重版权** - 下载内容请遵守相关法律法规
4. 🔒 **个人使用** - 请勿批量下载或分发

本软件仅为技术学习和个人使用而开发，使用者应当遵守相关法律法规。

## 📄 许可证

本项目使用 [MIT 许可证](./LICENSE) 开源。

## 🙏 致谢

- [egui](https://github.com/emilk/egui) - 优秀的 GUI 框架
- [aria2](https://aria2.github.io/) - 强大的下载工具
- [FFmpeg](https://ffmpeg.org/) - 专业的音视频处理
- [dddffgg](https://github.com/NORMAL-EX) - 软件开发及 1080P 分辨率视频免登录解析技术支持
- [hwyyds-skidder-team](https://github.com/hwyyds-skidder-team) - 720P 分辨率视频免登录解析技术支持
- 所有贡献者和用户的支持

## 📞 支持

如果你喜欢这个项目，请给它一个 ⭐！

- 🐛 [报告问题](../../issues)
- 💡 [功能建议](../../issues)
- 📖 [查看文档](../../wiki)

---


*最后更新：2026年2月7日*
