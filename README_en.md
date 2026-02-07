# Bilibili-Down

English | [简体中文](./README.md)

A modern Bilibili video downloader built with Rust and egui GUI framework.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)

## 👉 Official Website

https://bilibili-down.github.io

## 🌟 Features

- 🎥 **High-Quality Video Downloads** - Support up to 8K ultra-high definition
- ❌ **High-quality video download without login** - Breaking through Bilibili's wbi authentication enables login-free video downloads up to 1080P resolution.
- 🎵 **Audio Extraction** - Download MP3 format audio
- 🔐 **Login Support** - QR code login for premium video access
- ⚡ **Multi-threaded Downloads** - High-speed downloads powered by aria2c
- 🎨 **Modern Interface** - Intuitive and user-friendly GUI
- 🌐 **Multi-language Support** - Chinese/English interface switching
- 🎯 **Smart Parsing** - Support BV IDs and video URL parsing
- 📊 **Download Management** - Real-time progress display and queue management
- 🔄 **Auto Merging** - Intelligent video/audio merging

## 🚀 Quick Start

### System Requirements

- Windows / macOS / Linux
- Internet connection

### Installation

1. **Download Release**
   ```
   Download the latest version from Releases page
   ```

2. **Extract Files**
   ```
   Extract the archive to any directory
   ```

3. **Run Program**
   ```
   Double-click bilibili-down.exe to start
   ```

### Directory Structure

```
bilibili-down/
├── bilibili-down   # Main program
├── config.json     # Configuration file (auto-generated)
└── tools/
   ├── aria2c       # Download tool (included)
   └── ffmpeg       # Video processing tool (included)
```

## 📖 Usage Guide

### Basic Usage

1. **Start Program** - Double-click `bilibili-down`

2. **Input Video Information**
   - Support BV ID: `BV1xx411c7mD`
   - Support full URL: `https://www.bilibili.com/video/BV1xx411c7mD`
   - Support short URL: `https://b23.tv/xxxxx`

3. **Parse Video** - Click "Parse" button to get video information

4. **Select Quality** - Choose desired video quality in the popup window

5. **Start Download** - Select to download video or MP3 audio

### Advanced Features

#### Account Login
- Click the avatar icon in the top-right corner
- Scan QR code with your mobile phone to login
- After login, you can download high-definition videos (1080P+)

#### Download Management
- **Download Queue** - View all download tasks
- **Pause/Resume** - Control download progress
- **Delete Tasks** - Remove unwanted downloads

#### Settings
- **Theme Settings** - System/Light/Dark theme
- **Language Settings** - Chinese/English interface
- **Download Path** - Custom download directory
- **Thread Count** - Adjust download thread count

### Supported Video Quality

| Quality | Description | Login Required |
|---------|-------------|----------------|
| 8K | Ultra HD | ✅ |
| 4K | Super HD | ✅ |
| 1080P 60fps | High Frame Rate | ✅ |
| 1080P+ | High Bitrate | ✅ |
| 1080P | Full HD | ❌ |
| 720P | HD | ❌ |
| 480P | SD | ❌ |
| 360P | Low | ❌ |

## 🛠️ Development

### Build Environment

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone project
git clone https://github.com/NORMAL-EX/bilibili-down.git
cd bilibili-down

# Build project
cargo build --release

# Run project
cargo run
```

### Project Architecture

```
src/
├── main.rs              # Program entry point
├── app.rs               # Main application logic
├── config.rs            # Configuration management
├── bilibili.rs          # Bilibili API interface
├── downloader.rs        # Download manager
└── ui/
    ├── mod.rs           # UI module definitions
    ├── home.rs          # Home page interface
    ├── download_queue.rs # Download queue interface
    ├── settings.rs      # Settings interface
    ├── login.rs         # Login window
    └── video_detail.rs  # Video details window
```

### Tech Stack

- **GUI Framework**: [egui](https://github.com/emilk/egui) - Immediate mode GUI
- **HTTP Client**: [reqwest](https://github.com/seanmonstar/reqwest) - Async HTTP library
- **Async Runtime**: [tokio](https://tokio.rs/) - Async runtime
- **Serialization**: [serde](https://serde.rs/) - JSON serialization
- **Download Engine**: [aria2](https://aria2.github.io/) - Multi-threaded downloader
- **Video Processing**: [FFmpeg](https://ffmpeg.org/) - Audio/video processing

## 🤝 Contributing

Issues and Pull Requests are welcome!

### Contributing Guidelines

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Create a Pull Request

### Development Standards

- Follow Rust code style guidelines
- Add necessary comments and documentation
- Ensure all tests pass
- Update relevant documentation

## 📝 Disclaimer

**Important Notice**:

1. 🆓 **This software is completely free** - Beware of scams
2. 📚 **For learning and research only** - Do not use for commercial purposes
3. 🎯 **Respect copyright** - Follow relevant laws and regulations when downloading content
4. 🔒 **Personal use only** - Do not bulk download or distribute

This software is developed for technical learning and personal use only. Users should comply with relevant laws and regulations.

## 📄 License

This project is licensed under the [MIT License](./LICENSE).

## 🙏 Acknowledgments

- [egui](https://github.com/emilk/egui) - Excellent GUI framework
- [aria2](https://aria2.github.io/) - Powerful download tool
- [FFmpeg](https://ffmpeg.org/) - Professional audio/video processing
- [dddffgg](https://github.com/NORMAL-EX) - Software development and support for 1080P resolution video parsing without login
- [hwyyds-skidder-team](https://github.com/hwyyds-skidder-team) - Support for 720P resolution video parsing without login
- All contributors and users for their support

## 📞 Support

If you like this project, please give it a ⭐!

- 🐛 [Report Issues](../../issues)
- 💡 [Feature Requests](../../issues)
- 📖 [View Documentation](../../wiki)

---


*Last updated: December 7, 2025*
