# RepoJump 验收记录

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
