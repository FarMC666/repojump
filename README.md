# RepoJump

本地开发项目的快速启动器。添加代码根目录，搜索项目，按 Enter 在新的 VS Code 窗口中打开。

RepoJump 使用 Tauri 2、React、TypeScript 和 Vite，第一版面向 Windows 10/11 x64。无需账号，项目索引和用户设置仅保存在本机。

## 使用

1. 安装并启动 RepoJump，点击“添加代码根目录”，例如选择 `D:\code`。
2. 等待后台扫描，或直接搜索已经缓存的项目。
3. 输入名称、路径、分类或技术类型，使用上下键选择，按 Enter 打开 VS Code 新窗口。
4. 收藏常用项目；通过左侧“最近打开”找到之前打开的项目。

项目菜单支持终端、资源管理器、复制路径、仓库网页、收藏和分类覆盖。根目录外的项目、无标记的普通目录及 monorepo 子项目可通过“手动添加项目”加入。

“添加代码根目录”会扫描所选目录中的项目，例如选择 `D:\code` 后继续遍历 `apps`、`web`、`mods`，找到具体项目后停止深入。“手动添加项目”只添加选中的文件夹；如果没有检测到项目标记，会提供“扫描目录中的项目”和“仅添加此文件夹”两种方式。已经手动添加的分类目录也可以通过项目菜单中的“扫描目录中的项目”改为根目录，收藏、最近记录和分类覆盖保留。

添加或修改根目录后，主列表切回全部项目并清空旧查询。搜索支持 Windows 路径的两种分隔符，例如 `D:\code\mods` 和 `D:/code/mods`。

### 快捷键

| 快捷键 | 操作 |
| --- | --- |
| Ctrl+K | 聚焦搜索并选中当前查询 |
| ↑ / ↓ | 选择项目 |
| Enter | 在 VS Code 新窗口打开 |
| Esc | 关闭菜单或弹窗；清空查询；收起快捷启动窗口 |
| Ctrl+Alt+P | 全局唤起，清空查询并聚焦搜索 |

全局快捷键可在设置中修改或关闭。冲突会显示提示，不抢占其他程序的快捷键。默认关闭窗口后仍在托盘中运行；通过托盘菜单退出，或在设置中关闭驻留。通过全局快捷键唤起后，成功打开 VS Code 会自动收起窗口；普通主窗口会保留。

## 项目发现

识别 `.git`（目录或文件）、`package.json`、`pnpm-workspace.yaml`、`yarn.lock`、`package-lock.json`、`pyproject.toml`、`requirements.txt`、`Cargo.toml`、`go.mod`、`*.sln`、`*.slnx`、`*.csproj`、`pom.xml`、`build.gradle`、`build.gradle.kts`、`composer.json`、`Gemfile` 和 `*.code-workspace`。

一个项目可以具有多个技术标签，例如 JavaScript、TypeScript、React 和 Vite。分类来自代码根目录下第一层容器目录：`D:\code\apps\project` 自动归入 Apps。直接位于根目录中或根目录外的手动项目默认未分类。重叠根目录使用最具体的根目录分类，项目不会重复显示。

默认扫描深度为 4，根目录记作第 0 层，可在设置中调整为 1–8。**发现父项目后停止向下扫描**，子项目需要手动添加。目录符号链接和 junction 不参与递归；`node_modules`、`.git`、`dist`、`build`、`target`、`.next`、`.cache`、虚拟环境等生成目录会被跳过。

启动先加载缓存，再后台刷新。没有文件监听；新建项目或修改项目标记后，可点击“重新扫描”。不可访问的根目录保留缓存，失效项目标记不可用。删除根目录或手动记录只改变索引来源，不删除项目文件。

## 打开方式

- VS Code：使用 `Code.exe` 的 `--new-window` 参数，项目路径作为独立参数传递。优先使用设置中的路径，其次解析 PATH 中的 `code`，再检查注册表和常见安装位置。支持空格、中文及特殊字符路径。
- 终端：自动模式优先 Windows Terminal，回退至 PowerShell。PowerShell 通过进程工作目录进入项目，使用 `-NoProfile -NoExit`，不把项目路径插入脚本。带分号的路径走 PowerShell，避免 Windows Terminal 的命令分隔歧义。
- 仓库网页：读取 `origin`，支持 HTTP/S、标准 SSH、scp 风格 URL 和 Azure DevOps SSH 地址转换。不依赖 GitHub，也不执行网络 Git 操作。

Git 分支、修改状态和 origin 按需读取并短时缓存，需要本机可用的 Git。读取失败或超时只影响 Git 信息，发现和打开项目仍可使用。RepoJump 不修改项目内容，不执行安装依赖、pull、commit 或分支切换。

## 本地数据

位置可在设置中查看，默认是 `%LOCALAPPDATA%\com.farmc.repojump`。

- `state.json`：根目录、手动项目、收藏、最近记录、分类覆盖和设置。
- `state.json.bak`：上一个有效配置，用于恢复。
- `index.json`：可重建的项目发现缓存。

配置通过原子替换保存，带版本号。配置损坏时保留原文件，并优先从备份恢复；没有有效备份时明确提示并使用新配置。来自更新版本的配置会以只读方式保护，不覆盖原文件。应用升级使用固定数据目录，安装程序不清空这些数据。

## 开发

需要 Node.js 22.12+、Rust stable MSVC、Microsoft C++ Build Tools（Desktop development with C++，含 Windows SDK）和 WebView2。[Tauri Windows 前置要求](https://v2.tauri.app/start/prerequisites/)

```powershell
npm ci
npm run desktop
```

仅执行 `npm run dev` 是浏览器开发页面，会提示使用桌面程序；浏览器页面不会伪造本地项目能力。

```powershell
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run package
```

安装包位于 `src-tauri/target/release/bundle/nsis/`，打包工具缓存在 `src-tauri/target/.tauri/`。使用当前用户安装，不需要管理员权限。缺少 WebView2 时安装程序会下载其引导程序，因此首次安装可能需要联网；安装后核心功能不依赖网络。仓库不包含代码签名证书，默认构建的安装包未签名。[Tauri Windows 安装包说明](https://v2.tauri.app/distribute/windows-installer/)

模块边界见 [架构说明](docs/architecture.md)，实际检查与桌面验收范围见 [验收记录](docs/validation.md)。
