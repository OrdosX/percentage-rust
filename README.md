# Percentage Rust

This project is a Rust implementation of the [Percentage](https://github.com/kas/percentage) application. It explores dynamic rendering of tray icons using Rust and Tauri 2.

[中文 README](README.zh-cn.md)

## Features
- Dynamic tray icon rendering.
- Built with Rust as a learning project.
- Utilizes Tauri 2 for cross-platform desktop application development.

## Requirements
- Rust (latest stable version recommended)
- Tauri CLI

## Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/percentage-rust.git
   cd percentage-rust/src-tauri
   ```

2. Install dependencies:
   ```bash
   cargo install tauri-cli
   ```

3. Build the project:
   ```bash
   cargo tauri build
   ```

## Usage
Run the application:
```bash
cargo tauri dev
```

## Project Structure
- `src/`: Contains the Rust source code.
- `src-tauri/`: Contains Tauri configuration and assets.
- `assets/`: Fonts and other resources.

## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments
- Inspired by the original [Percentage](https://github.com/kas/percentage) project.
- Built with Rust and Tauri 2.