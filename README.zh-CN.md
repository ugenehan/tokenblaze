# TokenBlaze

[English](README.md) | [简体中文](README.zh-CN.md)

TokenBlaze 是一款 Windows 桌面伴侣，用于查看本机 AI 编程工具的使用情况。它读取受支持工具生成的本地会话文件或数据库，将解析出的用量事件保存在本地 SQLite 数据库中，并通过仪表盘和可选的动态火焰面板展示活动情况。

> 当前项目版本为 `0.1.0`。Token 数量和实时活动速率取决于各数据源实际提供的信息；部分数据（尤其是 Cursor 本地估算）属于估算值，不等同于账单记录。

## 目录

- [功能概览](#功能概览)
- [支持的数据源](#支持的数据源)
- [隐私与本地数据](#隐私与本地数据)
- [运行要求](#运行要求)
- [从源码运行](#从源码运行)
- [构建 Windows 安装包](#构建-windows-安装包)
- [仓库结构](#仓库结构)
- [常见问题](#常见问题)
- [当前限制](#当前限制)
- [许可证](#许可证)

## 功能概览

- **用量总览：** 查看今日 Token 总量、可用时的输入/输出/缓存拆分、数据源分布、每小时活动情况和最近七天趋势。
- **桌面火焰：** 可选的透明置顶桌面面板，根据近期活动显示火焰状态。仪表盘和火焰面板由同一个应用进程管理，并共享本地状态。
- **数据源状态：** 查看数据源是否已发现、最近读取时间，以及数据是否包含估算值。
- **外观设置：** 减少动态效果、暂停火焰动画、显示实时速率估算、调整火焰尺寸、按数据源设置颜色，并预览火焰样式。
- **系统托盘：** 从 Windows 托盘打开仪表盘、显示或隐藏火焰、暂停动画、检查更新或退出应用。
- **语言设置：** 应用界面支持跟随系统、英文、简体中文、日文和韩文。本仓库同时提供英文和简体中文 README，可通过文档顶部链接切换。
- **图表导出：** 将用量图表保存为 PNG 图片。

## 支持的数据源

当前版本从以下工具的本地数据中读取使用记录：

| 数据源 | 本地输入 | 说明 |
| --- | --- | --- |
| Claude Code | 会话文件 | 自动发现失败时，可在设置中填写会话目录。 |
| Codex | 会话文件 | 自动发现失败时，可在设置中填写会话目录。 |
| Cursor | 本地应用数据库及相关用量数据 | 根据本机可用数据，可能读取仪表盘记录或使用本地估算。可在数据源详情查看当前模式。 |
| Grok | 会话文件 | 自动发现失败时，可在设置中填写会话目录。 |
| Pi | 会话文件 | 自动发现失败时，可在设置中填写会话目录。 |
| Amp | 会话文件 | 自动发现失败时，可在设置中填写会话目录。 |
| OpenCode | 本地 SQLite 数据库 | 读取本地数据库，不需要调用 OpenCode 云端 API。 |

各工具的本地格式和安装目录可能随上游版本变化。工具尚未生成本地记录时，数据源可能显示为未连接。在**设置**中，可以为 Claude Code、Codex、Grok、Pi 或 Amp 填写会话目录；清空目录后会恢复自动发现。当前实现对 Cursor 和 OpenCode 使用自动数据库发现。

TokenBlaze 导入本地用量数据，但不会启动或控制这些数据源应用。它不保证与服务商账单或网页仪表盘完全一致。数据源可能缺少记录、延迟写入、提供累计值或估算值；只有数据源提供缓存字段时，界面才会展示缓存拆分。

## 隐私与本地数据

- 会话文件和数据源数据库从本机读取。
- 解析后的用量事件保存在本地 SQLite 数据库中；应用偏好设置由 Tauri 的本地存储插件保存。
- 用量监控功能不会上传会话或用量内容。
- **检查更新**需要网络连接，并读取本仓库 `main` 分支上的 Sparkle 格式 XML 清单 [appcast.xml](https://raw.githubusercontent.com/ugenehan/tokenblaze/main/appcast.xml)。清单需要包含由更新器公钥验证通过的 Windows 可执行文件，应用才能下载并安装更新。仓库发布页面为 [GitHub Releases](https://github.com/ugenehan/tokenblaze/releases)。
- 分享本地数据库或数据源会话文件前，请先检查内容。这些文件可能含有项目名称、提示词、文件路径或其他隐私信息。

## 运行要求

- 目标桌面平台为 Windows 10 或更高版本；当前 Tauri 安装包目标为 Windows NSIS。
- Rust stable 工具链和 Cargo。
- Node.js 与 npm，用于运行 Tauri CLI。
- Tauri 2 所需的 Windows 构建环境，包括 Microsoft C++ 构建工具和 WebView2 Runtime。

应用只读取本机已安装并使用过的工具所生成的数据。导入本地日志不需要 API Key 或服务商账号凭据。

## 从源码运行

在 PowerShell 中进入仓库根目录：

```powershell
cargo test --workspace
cargo check -p tokenblaze-app
```

在开发模式启动桌面应用：

```powershell
cd .\app
npm install
npm run dev
```

`npm run dev` 会启动 Tauri 应用，并从 `app/src` 加载前端。前端使用原生 HTML、CSS 和 JavaScript；npm 开发依赖只有 Tauri CLI。

## 构建 Windows 安装包

在 `app` 目录执行：

```powershell
npm install
npm run build
```

Tauri 配置会生成 NSIS 安装包。构建产物位于 `app/src-tauri/target` 下的 Tauri 输出目录；具体文件名取决于目标平台和构建配置。

从仓库根目录执行 Rust 验证：

```powershell
cargo test --workspace
cargo check -p tokenblaze-app
```

## 仓库结构

```text
.
├── app/
│   ├── src/                 # 仪表盘和火焰面板的 HTML、CSS、JavaScript
│   └── src-tauri/           # Tauri 桌面程序、窗口、托盘和命令
├── crates/
│   └── tokenblaze-core/     # 配置、数据源适配器、SQLite 存储、图表数据、
│                            # 多语言、火焰引擎和更新逻辑
├── Cargo.toml               # Rust 工作区和发布配置
├── Cargo.lock
├── LICENSE
├── README.md                # 英文文档
└── README.zh-CN.md          # 简体中文文档
```

Rust 工作区包含两个成员：`app/src-tauri`（`tokenblaze-app`）和 `crates/tokenblaze-core`（`tokenblaze-core`）。Tauri 进程统一管理仪表盘、火焰面板、托盘菜单、用量监控和共享应用状态。

## 常见问题

### 数据源未连接或没有用量记录

1. 先启动并使用对应工具，确认它已经生成本地会话或数据库记录。
2. 打开**设置**并选择**重新扫描数据源**。
3. 对 Claude Code、Codex、Grok、Pi 或 Amp，在设置中填写包含会话文件的目录。清空目录可恢复自动发现。
4. 确认当前 Windows 用户对所选目录或数据库有读取权限。

### Cursor 用量和 Cursor 仪表盘不一致

Cursor 数据接入会根据本机数据情况选择仪表盘记录或本地估算。无法读取精确记录时，估算值适合观察活动趋势，不应视作服务商账单数字。请查看仪表盘中的 Cursor 数据源详情，确认当前读取模式和估算状态。

### 火焰面板没有显示

在设置中启用火焰面板，或使用系统托盘菜单将其显示。暂停动画不会停止后台用量监控。如果仍未显示，可重启 TokenBlaze，并检查面板是否被全屏应用遮挡。

### Windows 下构建失败

确认 PowerShell 能找到 Rust/Cargo 和 Node.js/npm，并安装 Tauri 2 所需的 Windows 构建环境。如果原生依赖编译失败，先核对当前 Windows 工具链是否满足 Tauri 的构建要求，再考虑调整项目依赖。

## 当前限制

- 数据源发现依赖各工具的本地文件和数据库格式，上游格式变化可能导致适配失效。
- TokenBlaze 不会将本地统计与服务商账单进行核对。
- 缺少精确用量记录时，Cursor 可能显示估算值。
- 更新器指向本仓库 main/appcast.xml 清单和 GitHub Releases 页面。当前检出中尚无 appcast.xml，因此发布该清单前在线检查会失败。实际交付更新还需要有效的签名 Windows 版本条目。
- 当前安装包配置面向 Windows NSIS，尚未配置 macOS 或 Linux 安装包。

## 许可证

本项目采用 MIT License，详见 [LICENSE](LICENSE)。