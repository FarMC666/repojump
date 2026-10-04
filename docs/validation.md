# RepoJump 验收记录

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
