# RepoJump 验收记录

## 0.1.5：Windows CMD 终端选择（未发布更新）

2026-10-11，新增独立 Windows CMD 设置，Windows Terminal 选项明确标注使用默认 Shell。npm、Cargo 和 Tauri 统一保持 0.1.5，正式 bundle identifier 不变；本阶段修复合入同一未发布版本。

- typecheck、生产构建、56 项 Rust、19 项 Vitest、9 项 helper tests、fmt 和 Clippy all-targets / warnings as errors 通过。覆盖终端设置序列化/恢复、未知值拒绝、设置页 CMD 保存/再次打开、启动失败处理与 PowerShell 字面工作目录。
- 独立 Tauri 数据和 WebView2 中，通过真实设置页保存 CMD，再按 Ctrl+Enter。实际 `cmd.exe` 窗口保持运行，读取控制台提示符确认进入含中文、空格、`&`、括号、分号、`$` 和 `%PATH%` 的原始目录；目录中的百分号没有被展开。终端操作没有更新 Recent。
- 原生测试发现继承应用控制台/标准流会干扰 CMD 的交互生命周期。CMD 改用 Windows `CreateProcessW` 创建独立控制台，不继承应用句柄；可执行文件、固定 `/D /K` 参数、项目工作目录分别传递，不拼接用户 shell 命令，并关闭创建返回的进程/线程句柄。
- 真实 PowerShell 选项仍启动 PowerShell。独立应用重启后设置页恢复 CMD、两个项目保留、无存储告警。state schema 仍为 2、index schema 仍为 1；既有终端值保留，不进行隐式转换。
- 本机临时构建的 NSIS 打包及原位安装通过，安装期间 12 份配置/缓存/迁移备份 SHA256 不变。设置页保存 CMD，仅终端字段改变；实际 Ctrl+Enter 打开 `cmd.exe`，提示符位于选中项目，Recent 不变。正常退出与桌面重启后恢复 CMD、8 个项目及原有编辑器/Git Graph 设置，本地 recovery 与主状态一致。这是本机测试记录，未发布新版本。
- 本次没有实测 Windows Terminal 的默认 Shell 切换或 Windows 10；CMD 及 PowerShell 的本机流程使用 Windows 11。原生夹具及结果位于 ignored `.validation/terminal-016/`。
- 版本归一后重新完成六项检查和标准 NSIS 打包，并恢复本地安装为 0.1.5。主界面和设置页均显示 0.1.5，安装前后 12 份数据文件保持一致；8 个项目及用户当前保存的终端、编辑器和启动设置保留。结果及备份位于 ignored `.validation/version-015/`。

当前本地安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.5_x64-setup.exe`，1,942,364 字节。SHA256：`7E3A6C6376742E02D09AACB0EE9141904EB53C5AB682A86536233377C2F036FB`。此产物尚未发布。

## 0.1.5：Cursor 检测与启动反馈

2026-10-10，同步 npm、Cargo、Tauri 版本至 0.1.5；主界面和设置页从 package.json 读取版本，正式 bundle identifier 保持不变。

- typecheck、生产前端构建、53 项 Rust、18 项 Vitest、9 项 helper tests、fmt 和 Clippy all-targets / warnings as errors 通过。
- 实际 Cursor 位于自定义安装目录 `D:\apps\cursor`，安装记录为 `Cursor (User)`；CLI 位于 `resources\app\bin`。检测增加已知产品的 Windows 安装记录、32/64 位注册表视图及嵌套 CLI 路径映射，不执行 CLI 或注册表命令字符串。
- 独立 Tauri identifier / 数据 / Cursor profile 下，将 QA 进程 PATH 限制为 Windows 系统目录，仍自动检测到实际 Cursor executable；该次四编辑器检测约 73 ms。真实 Cursor 窗口打开了含中文、空格和 shell 字符的项目目录。
- 复现全局默认 VS Code Git Graph 被 Cursor 继承的场景：项目正常打开，UI 显示含 Cursor 名称的非错误说明，Recent 更新，Git Graph 设置保留。快捷模式在同样的能力降级下正常隐藏；helper 或 Recent 保存失败仍保留错误提示及窗口，Vitest 覆盖混合告警。
- Cursor 的 RepoJump helper / Git Graph capability 保持关闭。本次验证普通项目打开，不声称已验证相应高级能力。夹具、结果及安装备份保存在 ignored `.validation/cursor-015/`。
- 标准 NSIS 0.1.5 打包及本机 0.1.4 → 0.1.5 原位安装通过。安装阶段 12 份配置/缓存/迁移备份的 SHA256 不变；已安装的程序与 release 仅有预期的 NSIS bundle 标记差异，VSIX 一致，快捷方式指向安装目录。正常启动后主界面及设置页显示 0.1.5，保留 8 个项目、全局 Cursor、手动 executable 和 Git Graph 设置，恢复副本与当前状态一致。

## P0：文件监听、Editor Profiles 与 Action Palette

2026-10-10，在 0.1.4 工作树上实现本阶段功能，版本号及正式 bundle identifier 保持不变。使用独立 QA identifier、独立应用数据/WebView2 目录和 VS Code profile 运行真实 Windows/Tauri 验收；没有安装覆盖正式应用。

- `npm run typecheck`、`npm test`、`npm run build`、Rust tests、fmt check、Clippy all-targets / warnings as errors 通过。前端 14 项 Vitest、配套扩展 9 项 Node tests、Rust 50 项 tests；新增 jsdom 仅用于开发测试。
- 用 510 个项目及一个 root 外 manual 条目验证 cache-first 启动和搜索。新建、删除、重命名、marker 新增/移除、manifest 技术标签更新、`.git` 新增、manual manifest 更新均通过真实原生监听。
- `node_modules` 等 ignored directories 内生成文件和 500 次快速写入没有引发缓存重写；100 次 manifest 写入与多次手动 rescan 并发后最终标签正确。多 root 添加、修改、移除与正在运行的完整扫描没有发布过期来源。
- 真实 NTFS junction 夹具未被索引。原生测试复现了父子目录独立监听阻止 Windows 目录重命名的问题；改为合并 native subtree handles、按 discovery visited set 在入队前过滤后，父目录重命名测试与原生回归通过。
- 临时重命名隔离 root 模拟不可访问：缓存保留 missing 状态，收藏写入回退到本地 recovery；恢复目录后自动重新建立监听，随后 manifest 修改再次更新标签。真实进程重启恢复 roots、manual、收藏、分类、Recent、启动内容和编辑器设置，watcher 再次感知新项目。
- schema-1 夹具迁移后默认仍是 VS Code，旧启动内容和用户元数据保留。Rust 验证原始 `state.pre-v2.json` 不被后续保存覆盖、index schema 仍为 1、未知 Editor ID 保留、future schema 保护、自定义位置断开/重连恢复最新 Editor Profiles 和 overrides。
- 四种编辑器使用 Windows 原生探针 executable 验证 argument array、空格/中文/shell 字符、指定文件能力与普通打开降级、Recent、全局默认/项目覆盖/继承、临时 Open With、非法 executable 拒绝及数据保留。探针不代表对应厂商编辑器的真实打开行为。
- 本机实际安装的 VS Code 自动检测、普通启动和指定文件通过隔离 profile 验证；VS Code CLI、bundled VSIX 安装、generated workspace 和真实 Git Graph receipt 通过。Git Graph 扩展使用本机已安装版本的隔离副本；缺失扩展和超时路径返回警告，保留项目及 Recent。
- 真实 WebView2 检查 Tab、上下键、Enter、Esc、查询/选择恢复及设置展示；Vitest 额外覆盖 caret 恢复、disabled actions、菜单/Palette 共用执行逻辑、IME 和 repeated Enter guard。
- 通过真实 Windows Ctrl+Alt+Shift+F12 唤起隔离窗口，关闭已有 Palette 并恢复搜索；键盘 Open With 成功后收起 quick-launch，窗口隐藏期间 watcher 仍更新索引。真实 Close to tray 保留进程和 watcher；禁用该选项后正常关闭进程完成 native handle 清理。
- 实际将窗口客户区调整为 680×480，中文/深浅主题下 Palette 可滚动，设置底部按钮可见；截图经过检查。大窗口编辑器设置和 Open With 也经过检查。

本次没有可用的 VS Code Insiders、Cursor、Windsurf 安装，也没有真实 SMB/UNC share 或可拔插硬盘。对应 executable 的原生参数与配置测试已通过，但厂商 GUI、网络通知可靠性及硬件拔插仍需设备验收。目录重命名模拟不能替代这些测试。本次没有执行 NSIS 发布或安装升级验收。

隔离脚本、探针、夹具和截图保存在 ignored `.validation/p0/`。正式权限文件未增加通用 filesystem 或 shell 权限。

## 深色模式：石墨灰与柔和蓝

2026-10-04，按 [配色修改计划](dark-theme-plan.md) 更新深色主题，代码改动集中在 `src/styles.css`。

- `npm run build` 通过，包含配套扩展构建、TypeScript 检查及 Vite 生产构建；`git diff --check` 通过。
- 使用真实 Tauri/WebView2 调试窗口、独立应用标识和 `.validation/dark-theme/` 内隔离数据验证。主界面、项目菜单、设置、分类、VS Code 启动内容及移除确认弹窗，列表选中/悬停、键盘焦点、成功提示均通过检查。
- 实际调整原生窗口客户区至 1080×740、680×480，长名称和路径正确截断，项目列表与设置弹窗可滚动，底部操作区保持可见。深浅主题截图已逐张检查。
- 浅色主界面、搜索焦点、列表悬停、菜单与焦点项、设置弹窗及小窗口滚动状态，逐元素比较修改前后计算样式和尺寸，结果一致。
- 从设置切换深色、浅色和跟随系统均即时生效；页面刷新和真实进程重启后保留深色主题、26 个夹具项目及收藏。跟随系统的深浅变化使用媒体模拟验证；告警、忙碌、不可用项目和错误 Toast 使用模拟输入，Git 干净/未提交状态色使用临时 DOM 夹具验证。
- 主要按钮文字对比为 7.73:1，选中行与悬停行的辅助文字使用次要文字色，分别为 5.93:1、6.36:1；检查的正文颜色组合最低为 4.79:1。
- `npm run package` 通过，以代码提交 `b57392d7a7bcd86f201c1baba83f11c304100b75` 构建 Windows x64 NSIS 安装包。同版本 0.1.4 覆盖更新现有安装目录，安装过程未改动核对的 5 个用户数据文件，桌面与开始菜单快捷方式均指向更新后的程序。
- 真实安装版使用内置前端，深色主题的计算配色与计划一致，截图通过检查；切换深色并刷新后保留主题，随后恢复原有跟随系统设置。桌面快捷方式重启后，数据目录仍为 `D:\code\.repojump`，8 个项目、收藏、最近打开、分类覆盖和 VS Code 启动配置保持；用户状态逐字段相等，无存储告警或页面异常。安装后的程序除 Tauri NSIS 包类型标记外与 release 构建逐字节相同，配套 VSIX 完全相同。

本次安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.4_x64-setup.exe`，1,855,701 字节。SHA256：`253C5EE2E6DB7F3A312EA3C4BA8C00733E4BD8D07AA31504BACEDD8ABFD08170`。

截图和验收脚本保存在被忽略的 `.validation/dark-theme/` 和 `.validation/install-theme/`。本次验收范围为样式与主题行为，模拟状态检查不代表对应后端异常的真实复现。

## 0.1.4：全局默认 VS Code 启动内容

2026-10-04，在设置页加入默认启动内容，项目未单独配置时使用全局选项，已有的文件和 Git Graph 覆盖保持优先。

- TypeScript、生产前端构建、37 项 Rust、5 项 Vitest、9 项配套扩展测试、fmt 和 Clippy all-targets / warnings as errors 通过。
- 回归覆盖旧设置缺少默认启动字段、全局默认与项目覆盖的优先级、删除覆盖后继承、相对文件路径校验、持久化，以及全局默认随数据目录迁移和重启保留。
- 真实隔离桌面验收通过设置页输入非法路径时保持弹窗并提示、保存 `index.html` 后聚焦目标文件、已有项目文件覆盖优先、全局 Git Graph 出现在新窗口及删除项目覆盖后继承。默认文件不存在时只打开一个项目窗口并返回警告。
- 隔离桌面重启保留全局默认、项目覆盖、profile 与 Root 标识。原存储管理运行逻辑未修改。
- 标准 `npm run package` 通过；同版本 NSIS 覆盖更新现有快捷方式安装目录，安装后程序携带与构建输入一致的配套 VSIX，安装过程没有改动配置文件。
- 从桌面、开始菜单快捷方式分别打开更新后的安装版，设置页显示默认启动选项，数据目录仍为 `D:\code\.repojump`。保存不变设置与重启均无存储警告；原用户配置逐字段相等，只新增默认启动字段，已有项目的 Git Graph 配置保持。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.4_x64-setup.exe`，1,853,452 字节。SHA256：`1D8CDD4940097623919CA104DF8B2D1EA2AD4CAA129FD6CA3AE1A4E5C25F0274`。

## 0.1.4：复审后集成 VS Code 启动内容

2026-10-04，以恢复后的 0.1.4 为基准复审集成，保留数据位置迁移修复、托盘语言同步和原版本号。代码分析见 [集成审查](vscode-startup-review.md)。

- TypeScript typecheck、Vite production build、5 项 Vitest、9 项配套扩展测试、34 项 Rust 测试、fmt 和 Clippy all-targets / warnings as errors 通过。
- 新回归测试在第一次集成代码上复现旧配置缺少启动字段而触发 `storageLocationFallback`；修正后恢复原 profile 与 Root 标识，合并无冲突新增选项。同一项目选项冲突和其他用户记录有差异时，原保护继续生效。持久化测试确认 `profileId`、`dataLocation` 和原记录不因保存启动配置而丢失。
- 配置迁移回归覆盖启动内容在根目录、自定义目录与恢复位置之间保存和重启；等价配置恢复保留启动内容，只有启动内容不同的另一配置不会被误判为重复配置。
- 真实 Tauri/WebView2 验收使用隔离数据和 VS Code 配置。添加 Root 后为两个项目分别保存文件和 Git Graph，迁移到含应用自建 `.repojump` 的自定义位置，实际 `index.html` 聚焦和 Git Graph 目标窗口通过；已有文件窗口未切换。
- Git Graph 生成的工作区位于实际 AppData bootstrap 的 `vscode-launches/`，没有写入 Root 或自定义偏好目录。消费后请求与回执被清理，恢复自动配置位置仍保留工作区；真实桌面重启保留两个项目的启动内容和 Recent，没有存储告警。
- 使用真实 0.1.4 用户配置的只读副本重现历史重复 profile，实际恢复 Root、自定义位置迁移、文件聚焦、Git Graph 窗口和重启通过，原记录保留。
- 发布版普通工具子进程启动复现跨盘原子保存错误 `os error 17`，实际目录仍为 Root；改用 Explorer 文件夹视图代理启动后，Settings 显示 `D:\code\.repojump`，保存相同设置和重启均无存储告警。真实用户状态逐字段比较确认原配置完全保留，只增加空 `vscodeStartupOverrides`。
- 标准 `npm run package` 生成 Windows x64 release 和包含配套扩展的 NSIS 安装包。发布目录中的 VSIX 与构建输入 SHA256 一致；最终资源在全新隔离 VS Code 配置且 HTTP/S 代理不可用时离线安装通过。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.4_x64-setup.exe`，1,855,025 字节，约 1.77 MiB。SHA256：`51EE100EEA349DEF964407F9B901FD6BEA458CF337734BD2DA74410506EEB3E1`。

本次未重新执行安装、升级或 Windows 10 验收；已检查真实发布版普通桌面启动、保存与重启，新增启动内容的完整窗口流程使用调试版隔离配置。历史基础功能验收见下文。

## 按项目配置 VS Code 启动内容

2026-10-04，在 Windows 开发机上使用实际 Tauri/WebView2 窗口、VS Code 1.140.0 和 Git Graph 1.30.0 验证。应用数据和编辑器用户配置均使用工作区内的隔离目录。

- TypeScript typecheck、Vite production build、5 项 Vitest、9 项配套扩展测试、21 项 Rust 测试、Rust fmt 和 Clippy all-targets / warnings as errors 通过。
- 从项目菜单打开启动内容弹窗，非法相对路径保存失败并保留弹窗；原生文件选择器从目标项目开始，选中 `index.html` 后保存为相对路径。弹窗中的 Enter 不触发主列表启动。
- 实际按 Enter 后，含中文、空格、`&`、括号、分号和 `$` 的项目路径正确传给 VS Code；`index.html` 成为该项目窗口的活动编辑器。
- 配套扩展从随应用提供的 VSIX 安装或更新；存在其他项目窗口和同一项目的文件窗口时，Git Graph 仅在本次新工作区中显示。实际 Graph webview 显示目标仓库的 `Startup fixture` 提交。
- 请求消费后只留下工作区文件；重新加载该工作区不会再次消费请求。Windows 路径盘符和目录大小写差异另由扩展回归测试覆盖。
- 删除启动文件后仍打开项目并返回 `startupFileUnavailable`，Recent 更新成功。卸载隔离配置中的 Git Graph 后，返回 `gitGraphUnavailable` 并保存 Recent，没有重新安装 Git Graph。
- 启动文件删除且测试配置文件只读时，一次成功启动同时返回 `startupFileUnavailable` 和 `recentSaveFailed`；测试后恢复文件与权限。
- 恢复默认会删除项目覆盖；两个项目分别配置文件/Git Graph 后，真实桌面进程重启保留配置与 Recent。中英文、深浅主题的配置弹窗在 680×480 布局中无横向溢出或页面错误。
- 最终发布资源 `repojump-vscode.vsix` 在隔离 VS Code 配置且 HTTP/S 代理不可用的情况下可本地安装。标准 `npm run package` 构建 Windows x64 release 和包含配套扩展的 NSIS 安装包。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.1_x64-setup.exe`，1,830,666 字节，约 1.75 MiB。SHA256：`8A918AE92F7B18AADB6B079747C231EE6BF25EE108D08FA9CB2512ABB8EE0E24`。

本次图形流程使用调试版的隔离配置；没有重新执行发布版安装、升级或 Windows 10 验收。扩展禁用、请求超时、无信任工作区和重复消费由配套扩展测试覆盖。截图、脚本和结果位于被忽略的 `.validation/`。

## 0.1.4：重复迁移导致的数据位置回退

2026-10-04，在下述 Windows 11 开发机完成验证。Windows 10 未实际运行本次回归。

- 复现了普通桌面启动与 Codex 宿主启动访问不同 AppData 视图的问题。两份配置的用户记录一致，但 profile 和 Root 标识不同，0.1.3 将根目录数据误判为其他配置并回退。通过原生 Settings 确认实际目录为 AppData；独立读取未重定向的本机目录确认冲突原因。
- TypeScript typecheck、Vite production build、5 项 Vitest、25 项 Rust 测试、Rust fmt、Clippy all-targets / warnings as errors 和 Windows x64 NSIS 构建通过。新增测试覆盖等价重复配置恢复、无位置记录的旧配置识别已有较新数据、真正不同的用户记录仍受保护，以及恢复首选目录后清除过期回退提示。
- 独立配置在真实 Tauri/WebView2 窗口中恢复到包含中文、空格和特殊字符路径的根目录，完成后台扫描、Settings 保存及重启。根目录标识、收藏和 Recent 保留，没有存储告警。
- NSIS 0.1.3 → 0.1.4 覆盖升级退出码 0；安装阶段根目录及普通桌面 AppData 配置的 SHA256 不变。升级后通过 Windows 资源管理器启动实际安装程序，Settings 显示 `D:\code\.repojump`，默认本地恢复副本与位置记录使用根目录中的原配置标识。重复迁移产生的旧 AppData 配置已在工作区内备份。
- 实际安装版保存相同设置并再次正常桌面启动，未再出现回退或保存错误；根目录原用户状态 SHA256 保持一致。源配置中的 5 条 Recent、分类覆盖和已有设置保留；内部 Root 标识复用根目录原记录。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.4_x64-setup.exe`，1,813,190 字节，约 1.73 MiB。SHA256：`4788465E27F65F5AEF53821D6E9E1A7D396EF0A886518D1DF1BE24587977D4C9`。

本次未重新验证未修改的 VS Code、Terminal、Explorer 和 Git 启动逻辑。

## 0.1.3：托盘菜单语言

2026-10-04，在下述 Windows 11 开发机完成验证。Windows 10 未实际运行本次回归。

- TypeScript typecheck、Vite production build、5 项 Vitest、21 项 Rust 测试、Rust fmt、Clippy all-targets / warnings as errors 和 Windows x64 NSIS 构建通过。
- 在独立配置中通过真实 Settings 保存中文和英文，Windows 原生托盘菜单立即分别显示“打开 RepoJump / 退出”和“Open RepoJump / Quit”，每个选项只显示一种语言。原生菜单截图完成检查。
- 跟随系统与主界面语言一致。模拟 WebView 语言与 Windows 显示语言不同，托盘跟随主界面解析结果；过期的同步请求不会覆盖显式语言设置。
- 通过托盘运行时的原生右键通知消息打开菜单，验证打开和退出选项的回调。重新启动独立配置后保留英文菜单；最终安装版验证主界面与原生托盘菜单均为中文，原用户配置 SHA256 不变。未单独执行托盘图标上的物理鼠标右键测试。
- NSIS 0.1.2 → 0.1.3 覆盖升级退出码 0，安装阶段原用户配置 SHA256 不变。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.3_x64-setup.exe`，1,811,902 字节，约 1.73 MiB。SHA256：`904052AFB4BFBDEA2003FB554491BDE301BC0171BF1752598155B5CEE24C5053`。

## 0.1.2：本地数据位置与滚动行为

2026-10-04，在下述 Windows 11 开发机完成验证。Windows 10 未实际运行本次回归。

- TypeScript typecheck、Vite production build、5 项 Vitest、20 项 Rust 测试、Rust fmt 和 Clippy all-targets / warnings as errors 通过；Windows x64 release 和 NSIS 构建通过。
- 存储测试覆盖首次启动的默认隐藏目录、第一个 Root、后续 Root 位置稳定、自定义目录、恢复自动位置、移除全部 Root、旧配置迁移、重启、运行期间离线回退及重连后保留新设置。另覆盖位置记录损坏保留、迁移目标配置冲突、位置记录写入失败时保留源数据，以及新版本配置只读保护。
- 独立配置通过真实 Root 选择器添加含空格、中文、`&`、括号、分号的目录，发现 45 个项目，数据进入该 Root 的 `.repojump`。第二个 Root 不改变位置；在 `.repojump` 中放置带项目标记的测试目录也不会被扫描。
- 真实 Settings 目录选择器验证 Cancel 不迁移、Save 迁移并保留收藏和分类、旧位置副本保留。选择已有其他 profile 的目录显示错误且不覆盖双方数据。
- 真实搜索 → Enter → VS Code 新窗口 → Recent 保存通过。正常退出并重启后，自定义位置、Roots、收藏、分类和 Recent 恢复。模拟数据盘在运行期间离线后仍可保存；重连并重启后接收离线期间的新设置，Recent 保留。
- 真实 WebView 列表滚到底后继续发送滚轮输入，位置保持在边界；CSS `overscroll-behavior: none` 生效。Windows WebView2 进程确认使用 `--disable-features=ElasticOverscroll`。中英文、深浅主题和 680×480 视口中的设置页无横向溢出，截图完成检查。
- 旧 AppData 配置迁移后，用户原有 Root、手动条目、收藏、5 条 Recent、分类覆盖和已有设置逐字段一致。最终安装版数据目录为 `D:\code\.repojump`，Hidden 属性生效，当前深度 2 的 Root 扫描发现 8 个项目。
- NSIS 覆盖安装退出码 0；安装阶段原配置不变。最终安装程序正常启动和再次重启后，Settings 显示新目录，保存相同设置成功，没有存储告警；搜索及滚动边界通过。最终运行实例已移除临时远程调试参数。
- 本机 Windows 打包宿主会将 AppData 新写入重定向到另一磁盘。位置记录使用新建恢复文件夹的实际父目录，真实安装版验证原子替换不再出现跨磁盘移动错误。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.2_x64-setup.exe`，1,806,054 字节，约 1.72 MiB。SHA256：`FC2F519F82F8DF8125B1D38B20964EDFBDF789A66363DDD4245E0C2FF0F36C4F`。

本次没有重新验证未修改的 Terminal、Explorer、托盘和全局快捷键流程；基础验收见下文。滚轮边界和启动参数已验证，未单独模拟触控板惯性手势。

## 0.1.1：分类目录发现与搜索回归

2026-10-04，在下述 Windows 11 开发机上完成验证。Windows 10 未实际运行本次回归。

- TypeScript typecheck、Vite production build、5 项 Vitest、15 项 Rust 测试、Rust fmt 和 Clippy all-targets / warnings as errors 通过。
- 通过真实原生 Root 选择器添加 `D:\code`，发现 9 个项目，其中 `mods` 下的两个 Git 项目均被识别。选择 `mods` 作为 Root 发现这两个项目；通过设置切换回 `code` 恢复全部 9 个项目。
- 添加 Root 前停留在收藏视图并输入无匹配查询；添加成功后自动切回全部项目并清空查询，项目可见。
- 手动选择无标记的分类目录时出现扫描/仅添加选择。选择扫描已存在的 Root 不产生重复；将已有手动目录转为 Root 时保留收藏与分类覆盖。最近记录保留另由 Rust 测试覆盖。
- 手动添加已有 Root 覆盖的 Git 项目只增加手动来源，不重复显示。Git 信息不参与项目发现是否成功的判断。
- 实际安装后的发布版搜索 `sil` 命中 Silent-Translator；`D:/code/mods` 命中两个 Git 项目。独立测试配置中确认 `D:\code\mods` 与 `D:/code/mods` 查询结果一致。
- 检查中英文、深浅主题和 680×480 布局，没有页面横向溢出或 WebView 页面错误。
- 0.1.0 → 0.1.1 NSIS 覆盖升级退出码 0；升级前后用户配置 SHA256 一致。重新打开实际安装程序后保留原有根目录、手动条目、最近记录及分类覆盖，并显示新增 Root 发现的项目。实际用户列表为 9 个自动发现项目及 2 个原有手动目录条目。

安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.1_x64-setup.exe`，1,791,910 字节，约 1.71 MiB。SHA256：`3B78F4DB7806321FFEA87150ABDC6774CEE5734DCD0A2A7C18B5746A1E16BD72`。

本次没有重新验证 VS Code、终端、Explorer 等未修改的启动逻辑；其基础验收记录如下。

## 0.1.0：基础验收

验证日期：2026-10-04。开发机为 Windows 11 专业工作站版 x64，系统版本 `10.0.26200`，WebView2 `154.0.4258.53`。Windows 10 未实际运行验收。

## 自动检查

| 检查 | 结果 |
| --- | --- |
| TypeScript typecheck、Vite production build | 通过 |
| Vitest | 4 项通过：搜索字段、模糊与多词匹配、排序、500 项目性能 |
| Rust tests | 13 项通过：全部标记、多标签、父项目截断、深度、手动嵌套、路径和分类、来源保留、不可用 Root、远程 URL、字面参数、配置恢复与版本保护、实际数据目录解析 |
| Rust fmt、Clippy all-targets / warnings as errors | 通过 |
| npm 生产依赖审计 | 0 个已知漏洞 |
| 标准 `npm run package` | Windows x64 release 与 NSIS 构建通过 |

## 真实桌面验证

测试通过实际 Tauri/WebView2 桌面窗口进行，包含 Windows 原生文件夹对话框、真实 VS Code、PowerShell、Explorer 和浏览器调用；没有使用模拟项目后端。临时脚本、项目和截图位于被忽略的 `.validation/`。

| 流程 | 结果 |
| --- | --- |
| 首次启动 → 原生选择 Root → 后台扫描 → 搜索 → Enter → VS Code → Recent | 开发版和安装后的发布版通过；发布版使用打包前端 |
| 项目发现与类型 | 自动发现 505 个项目；手动普通文件夹及嵌套项目加入后共 507 个；忽略生成目录，识别 React / TypeScript / Vite 等多标签 |
| Root、分类和来源 | 重叠 Root 去重、最具体 Root 分类、分类覆盖、移除来源、扫描中修改 Root 均通过 |
| 异常与恢复 | 真实 ACL 拒绝和 Root 离线保留缓存；目录删除标记失效；启动失败不更新 Recent；恢复目录后重新可用 |
| junction 循环 | 创建指回 Root 的 junction 后仍只访问 513 个目录、发现 505 个项目，未递归进入循环 |
| 空格、中文、特殊字符 | 对应 VS Code 窗口的原始标题确认 `Test Project`、`测试项目`、`a & (b); $c`；Explorer 定位特殊字符目录；复制路径与绝对路径一致 |
| Git | 正常仓库读取 branch、dirty、GitLab SSH origin 并打开对应 HTTP/S 地址；外部所有权检查失败时仍能搜索和打开项目 |
| 桌面生命周期 | 实际全局按键唤起、搜索聚焦、快捷打开后隐藏、普通打开保留、关闭到托盘、托盘唤起和退出、再次启动唤起已有实例均通过 |
| 快捷键冲突 | 实际占用测试组合后保存失败，原组合和配置保留；本机默认 Ctrl+Alt+P 已被占用，使用 Ctrl+Alt+Shift+J 完成验证 |
| 键盘与界面 | Ctrl+K、上下选择、中文输入组合事件保护、中英文、深浅主题和 680×480 布局通过 |
| 重启 | 507 项目缓存、设置、Favorites 和 Recent 恢复通过 |
| 安装 | 当前用户静默安装退出码 0；最终安装包同版本覆盖安装退出码 0；用户配置字节不变，重启后 Roots / 设置 / Favorites / Recent 一致 |

真实路径测试使用工作区内等价目录，不创建或覆盖用户的 `D:\code\test`、`D:\My Code\Test Project`、`D:\代码\测试项目`。这些原始示例另由 Rust 参数测试验证作为一个完整参数传递。

500 项目搜索经过 10 次预热和 100 次本地测量，P95 为 **18.99 ms**，最大 38.93 ms。此数值衡量内存搜索函数，不代表 UI 绘制或扫描耗时。

安装后的数据目录解析验证覆盖了 Windows 打包宿主的 AppData 路径重定向：保存和有效备份使用同一实际目录完成原子替换。验收前正式数据目录无用户配置；测试配置已归档，并恢复空白首次启动状态。

## 交付产物与验证边界

- 安装包：`src-tauri/target/release/bundle/nsis/RepoJump_0.1.0_x64-setup.exe`，1,787,852 字节，约 1.71 MiB。
- SHA256：`BB7E1B7A1CB14540708B9AC6C57672C887D1DE926FA077D26E998933EA51683A`。
- 发布程序：`src-tauri/target/release/repojump.exe`。
- 安装包未签名；WebView2 缺失时的引导下载安装未在本机执行，因为本机已有运行时。
- Windows Terminal 的 `wt.exe` 分支未在本机实际调用；本机实测了自动回退 PowerShell。未把 Windows 10、其他机器或跨版本升级标记为通过。

配置备份恢复、损坏文件保留、无版本配置迁移和更新版本只读保护由 Rust 测试验证；同版本安装覆盖由实际 NSIS 安装验证。
