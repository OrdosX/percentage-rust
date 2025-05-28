# Percentage Rust

本项目是 [Percentage](https://github.com/kas/percentage) 项目的 Rust 实现，探索了使用 Rust 和 Tauri 2 动态渲染托盘图标的方法。

## 特点
- 动态托盘图标渲染。
- 可作为初学 Rust 的参考。
- 利用 Tauri 2 开发跨平台桌面应用。

## 环境要求
- Rust（建议使用最新稳定版本）
- Tauri CLI

## 安装步骤
1. 克隆仓库：
   ```bash
   git clone https://github.com/your-username/percentage-rust.git
   cd percentage-rust/src-tauri
   ```

2. 安装依赖：
   ```bash
   cargo install tauri-cli
   ```

3. 构建项目：
   ```bash
   cargo tauri build
   ```

## 使用方法
运行应用：
```bash
cargo tauri dev
```

## 项目结构
- `src/`：包含 Rust 源代码。
- `src-tauri/`：包含 Tauri 配置和资源。
- `assets/`：字体和其他资源。

## 许可证
本项目使用 MIT 许可证。详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢
- 项目灵感来源于原始 [Percentage](https://github.com/kas/percentage) 项目。
- 使用 Rust 和 Tauri 2 构建。
