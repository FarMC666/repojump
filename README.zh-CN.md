# RepoJump

[English](README.md) | [简体中文](README.zh-CN.md)

轻量的 Windows 本地开发项目启动器。添加代码根目录，找到项目，按 **Enter** 使用配置的编辑器打开。

使用 **Tauri 2、React、TypeScript 和 Vite** 构建，面向 Windows 10/11 x64。本地运行，无需账号。

## 快速开始

1. 安装并启动 RepoJump。
2. 点击**添加代码根目录**，选择 `D:\code` 等存放代码的目录。
3. RepoJump 在后台扫描。通过项目名称、路径、分类或技术类型搜索项目。
4. 使用上下键选择项目，按 **Enter** 打开。

```text
D:\code
├── apps
│   ├── desktop-tool
│   └── another-app
├── web
│   └── website
└── mods
    └── game-mod
```

选择 `D:\code` 后，会继续遍历 `apps`、`web`、`mods` 等容器目录，发现其中的项目。这些目录也用于自动分类。添加或修改根目录后，主列表切回全部项目并清空之前的查询。

根目录外的项目、没有项目标记的普通文件夹，以及 monorepo 中的嵌套项目，可以通过**手动添加项目**加入。该操作只添加选中的文件夹；未找到项目标记时，会提供将其作为根目录扫描的选项。已有手动条目也可以通过菜单中的**扫描目录中的项目**转为根目录，收藏、最近记录和分类覆盖会保留。

## 功能

- 即时本地搜索，支持名称模糊匹配和多词查询。Windows 路径支持 `\` 和 `/` 两种分隔符。
- 收藏项目置顶；独立维护最近打开记录，不依赖 VS Code 的历史数据。
- 项目菜单和键盘 Action Palette 共用编辑器、终端、资源管理器、复制路径、仓库网页及收藏动作。
- 后台发现项目，先加载缓存，自动监听相关文件变化，支持调整扫描深度和手动重新扫描。
- 可修改的全局快捷键、托盘菜单和单实例唤起。
- 深色、浅色和跟随系统主题；英文和简体中文界面。
- 本地 JSON 存储，支持备份和自定义数据位置。

## 快捷键

| 快捷键 | 操作 |
| --- | --- |
| Ctrl+K | 聚焦搜索并选中当前查询 |
| ↑ / ↓ | 选择项目 |
| Enter | 使用项目覆盖或全局默认编辑器打开 |
| Tab | 打开所选项目的动作面板；↑/↓ 选择，Enter 执行，Esc 返回搜索 |
| Ctrl+Enter | 打开终端 |
| Alt+Enter | 打开资源管理器 |
| Esc | 关闭菜单或弹窗、清空查询，或收起快捷启动窗口 |
| Ctrl+Alt+P | 唤起 RepoJump、清空查询并聚焦搜索 |

全局快捷键可在设置中修改或关闭。发生冲突时保留之前的设置并显示错误。RepoJump 需要保持运行才能响应全局快捷键。

默认关闭窗口后继续在托盘中运行。可以通过托盘菜单退出，或在设置中关闭此行为。通过全局快捷键唤起的窗口，在成功打开编辑器且没有警告时自动收起；普通方式打开的主窗口会保留。

设置中的主题和语言修改后立即生效并自动保存，取消也会保留；其他设置仍需点击“保存”。托盘菜单跟随软件语言设置，包括跟随系统；切换语言后立即更新。

## 项目发现

支持识别以下项目标记：

```text
.git（目录或 worktree 文件）
package.json       pnpm-workspace.yaml   yarn.lock       package-lock.json
pyproject.toml     requirements.txt      Cargo.toml      go.mod
*.sln              *.slnx                *.csproj        *.code-workspace
pom.xml            build.gradle         build.gradle.kts
composer.json      Gemfile
```

一个项目可以具有多个技术标签，例如 JavaScript、TypeScript、React 和 Vite。其他检测规则覆盖 Next.js、Python、Rust、Go、.NET、Maven/Gradle、PHP/Composer 和 Ruby。配置文件损坏不会阻断项目发现。

默认最大扫描深度为 **4**，根目录记作第 **0** 层，可在设置中调整为 1–8。**识别到项目后停止扫描其子目录**，嵌套项目需要手动添加。不递归进入符号链接或 junction；跳过 `node_modules`、`.git`、`dist`、`build`、`target`、`.next`、虚拟环境、`vendor`、`.repojump` 等生成或数据目录。

自动分类来自最具体的匹配根目录下的第一层容器目录。直接位于根目录中的项目、根目录自身，以及根目录外的手动项目归入未分类。重叠根目录和手动来源不会导致项目重复显示。

启动时先显示缓存，再后台刷新。文件监听会在相关变化后更新项目及技术标签，并在后台合并短时间内的事件。**重新扫描**仍是完整扫描兜底。暂时断开的根目录和失败的监听约每 30 秒重试，网络目录也会定期核对。无法读取的根目录保留缓存条目，已删除目录标记为不可用。移除根目录或手动条目只改变索引来源。

## 打开项目

在**设置 → 打开方式**启用 **Visual Studio Code、Visual Studio Code Insiders、Cursor 或 Windsurf**，选择全局默认编辑器。每个 Profile 支持自动检测或手动选择可执行文件。在项目菜单的**项目编辑器**中可覆盖全局默认；选择跟随全局设置会删除覆盖。菜单或动作面板的**使用其他编辑器打开**仅影响本次启动。可执行文件失效时明确提示，保留项目数据。

| 编辑器 | 打开项目 | 指定启动文件 | RepoJump helper / Git Graph |
| --- | --- | --- | --- |
| VS Code | 支持 | 支持 | 支持 |
| VS Code Insiders | 支持 | 支持 | 暂不支持 |
| Cursor | 支持 | 暂不支持 | 暂不支持 |
| Windsurf | 支持 | 暂不支持 | 暂不支持 |

自动检测检查 PATH、App Paths、Windows 安装记录（包括自定义安装目录）和常见路径，支持 `bin` 与 `resources/app/bin` 两种 CLI 目录结构，不执行 CLI 脚本。

不支持的启动配置会保留；这些编辑器正常打开项目后显示说明，不作为错误，也不阻止快捷启动自动收起。不会执行未经验证的 helper 或 Git Graph 命令。Enter、双击、打开按钮及动作面板共用启动逻辑，成功打开任一编辑器后均更新 Recent。

**VS Code：**使用 `Code.exe` 的 `--new-window` 参数，项目路径作为独立参数传递。优先使用设置中指定的可执行文件，其次检查 PATH、注册表和常见安装位置。自动检测失败时，可在设置中指定路径。空格、中文和 shell 特殊字符均作为路径的一部分处理。

在设置页的**打开方式 → 默认 VS Code 启动内容**中，可选择默认打开、指定项目内的相对文件路径或 Git Graph，作用于未单独配置的项目。全局文件路径在启动时按每个项目分别检查，文件不存在时仍打开项目并提示。

在项目菜单的**VS Code 启动内容**中，可以为项目单独配置启动选项，优先于全局默认。Enter、双击和打开按钮统一应用该配置：

- **跟随全局设置**：使用设置页的默认启动内容。
- **指定文件**：输入项目内文件的相对路径（例如 `index.html`），或使用原生文件选择器。文件必须已存在且位于项目内，启动后成为活动编辑器。
- **Git Graph**：在本次新开的项目窗口显示 Git Graph。需要预先安装并启用 `mhutchie.git-graph`；RepoJump 首次使用时自动安装随应用提供的配套扩展。此模式使用应用数据目录中的独立工作区，不修改项目文件。只有本次 RepoJump 请求会触发显示，手动打开项目或恢复已消费的工作区不会重复执行。

文件失效、配套扩展安装失败、Git Graph 不可用或超时，仍保留打开的项目窗口并提示。原配置保留，Recent 仍会保存；选择跟随全局设置会删除该项目的覆盖。

**终端：**在设置中选择 Windows CMD、PowerShell、Windows Terminal 或自动，然后保存。Windows CMD 直接通过 `cmd.exe /D /K` 在项目目录中打开命令提示符，并禁用注册表 AutoRun。Windows Terminal 使用它配置的默认 Shell，可能是 PowerShell。自动模式优先 Windows Terminal，其次 PowerShell。CMD 和 PowerShell 通过进程工作目录进入项目，不会将项目路径拼入 shell 脚本。自动和 Windows Terminal 模式下，包含分号的路径使用 PowerShell，避免 Windows Terminal 命令分隔符产生歧义。

**仓库网页：**项目菜单读取 Git 的 `origin`，将支持的 HTTP/S、SSH 和 scp 风格地址转换为浏览器链接，并保留原仓库主机。支持 GitHub、GitLab 等主机，并提供 Azure DevOps SSH 地址的专门转换。

Git 元数据为可选信息，按需读取，使用短时缓存和进程超时。Git 读取失败不影响项目发现和打开。RepoJump 不安装依赖、不切换分支，也不执行 pull、commit 或 push；只写入自己的应用数据。

## 本地数据

默认在**第一个添加的代码根目录**中创建隐藏的 **`.repojump` 文件夹**：

```text
D:\code\.repojump\
├── state.json
├── state.json.bak
└── index.json
```

没有根目录，或该位置不可用时，回退到：

```text
%LOCALAPPDATA%\com.farmc.repojump\.repojump
```

之后添加的根目录不会改变自动数据位置。移除第一个根目录后，使用下一个根目录；没有剩余根目录时使用默认位置。修改第一个根目录时，数据迁移到其新路径。

在**设置 → 本地数据**中选择**自定义目录**，指定父目录并保存。RepoJump 在其中创建 `.repojump`，迁移根目录、手动条目、收藏、最近记录、设置、分类覆盖和缓存。**当前数据目录**显示实际使用的路径。恢复自动位置后重新使用第一个根目录；取消设置不会切换位置。

- `state.json`：用户状态，包含按项目保存的 VS Code 启动内容覆盖、配置版本和配置标识。
- `state.json.bak`：上一个有效配置。
- `index.json`：可重建的项目缓存。

原来的 `%LOCALAPPDATA%\com.farmc.repojump` 目录保留小型位置记录 `storage-location.json` 和一份 `.repojump` 恢复副本，让代码盘断开时应用仍能启动。在回退期间保存的更改，会在配置的数据盘恢复并重启 RepoJump 后保留。WebView2 自身的运行时数据仍位于系统应用数据目录。

Git Graph 的工作区始终保存在该应用数据目录的 `vscode-launches/` 下，不随用户配置迁移。请求与回执在消费或过期后清理；工作区文件保留供窗口恢复，并且不会加入 VS Code 最近打开列表。

迁移先写入新数据，再切换位置记录，并保留旧副本。目标目录属于其他配置时会拒绝迁移。旧版仅使用 AppData 的配置会自动迁移。配置通过原子替换保存；损坏文件会保留，有效备份会恢复并显示提示。不支持的更新版本配置受到写入保护。安装程序升级保留应用数据。

如果不同启动环境重复迁移了同一份配置，自动存储会在用户记录一致时复用根目录中已有的数据。记录有差异或自定义目录中的配置冲突仍受到保护；恢复首选目录后不会保留过期的回退提示。

`.repojump` 设置了 Windows 隐藏属性，可在资源管理器中启用**隐藏的项目**查看。如果代码根目录本身是 Git 仓库，请自行将 `.repojump/` 加入忽略规则，避免提交本地偏好；RepoJump 不修改项目的 `.gitignore` 文件。

用户状态 schema 升级为 **2**：旧 VS Code 路径迁移到 VS Code Profile，默认编辑器保持 VS Code，原启动覆盖及其他偏好全部保留。index 和 locator schema 保持 **1**。迁移前保存原始 `state.pre-v2.json`。降级前退出 RepoJump，备份当前活动数据和本地 recovery copy，再成套恢复迁移前状态与匹配的 locator；旧版不能写入 schema-2 状态。

## 开发

需要 Node.js 22.12+、Rust stable MSVC 工具链、Microsoft C++ Build Tools（**使用 C++ 的桌面开发**，包含 Windows SDK）以及 WebView2。详见 [Tauri 官方 Windows 前置要求](https://v2.tauri.app/start/prerequisites/)。

```powershell
npm ci
npm run desktop
```

`npm run dev` 只启动浏览器前端。本地项目访问需要桌面程序，浏览器页面不会模拟原生功能。

```powershell
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run package
```

Windows x64 NSIS 安装包输出到 `src-tauri/target/release/bundle/nsis/`，打包工具缓存在 `src-tauri/target/.tauri/`。使用当前用户安装，无需管理员权限。缺少 WebView2 时安装程序会下载引导程序，这一步需要联网；安装后核心功能可离线运行。未提供代码签名凭据时，构建产物未签名。详见 [Tauri Windows 安装包文档](https://v2.tauri.app/distribute/windows-installer/)。

模块边界见[架构说明](docs/architecture.md)，实际执行的检查和平台范围见[验收记录](docs/validation.md)。

配套扩展位于 `vscode-helper/`，`npm run build:helper` 使用 Node.js 内置库生成 `src-tauri/resources/repojump-vscode.vsix`。桌面开发、生产构建和直接 Cargo 构建都会生成此资源；安装包包含它，支持离线安装。
