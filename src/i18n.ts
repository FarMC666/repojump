import type { AppError, Settings } from './models';

const en = {
  all: 'All projects', favorites: 'Favorites', recent: 'Recent', categories: 'Categories', uncategorized: 'Uncategorized',
  search: 'Search projects…', settings: 'Settings', addRoot: 'Add code root', addProject: 'Add project', rescan: 'Rescan',
  projects: 'projects', searching: 'Search results', scanning: 'Scanning', ready: 'Ready', visited: 'folders checked',
  welcome: 'Your projects, one shortcut away.', welcomeBody: 'Add a folder that contains your code. RepoJump will find your projects automatically.',
  noResults: 'No matching projects', noResultsBody: 'Try a project name, path, category or technology.',
  noFavorites: 'Keep your go-to projects here', noFavoritesBody: 'Star a project to keep it at the top of your list.',
  noRecent: 'Your next project starts here', noRecentBody: 'Projects opened in VS Code appear here automatically.',
  noProjects: 'No projects found', noProjectsBody: 'Add a project folder manually, change the scan depth or choose another code root.',
  desktopOnly: 'Open RepoJump as a desktop app to access your projects.', loading: 'Loading your project index…', retry: 'Retry',
  openCode: 'Open in VS Code', openTerminal: 'Open in Terminal', openExplorer: 'Open in Explorer', copyPath: 'Copy path', openRepository: 'Open repository',
  favorite: 'Favorite', unfavorite: 'Unfavorite', more: 'Project actions', copied: 'Path copied', category: 'Category', editCategory: 'Change category', autoCategory: 'Restore automatic category',
  removeManual: 'Remove manual entry', removeManualBody: 'Only the manual entry is removed. The project folder is kept. Projects discovered by a root remain in the list.',
  manual: 'Manually added', missing: 'Folder missing', unknown: 'Availability unknown', git: 'Git repository', branch: 'Branch', clean: 'Working tree clean', dirty: 'Uncommitted changes', gitUnknown: 'Git information unavailable',
  lastOpened: 'Last opened', never: 'Not opened yet', open: 'Open', cancel: 'Cancel', save: 'Save', remove: 'Remove', close: 'Close',
  roots: 'Code roots', rootsBody: 'RepoJump scans these folders. Finding a project stops recursion into its children.', noRoots: 'No code roots yet.',
  changeRoot: 'Change root folder', removeRoot: 'Remove root', removeRootBody: 'Remove this root from RepoJump? Its project folders stay untouched. Manual entries and other roots remain.',
  appearance: 'Appearance', theme: 'Theme', dark: 'Dark', light: 'Light', system: 'Follow system', language: 'Language',
  discovery: 'Discovery', scanDepth: 'Maximum scan depth', depthHint: 'The root is depth 0. Default: 4. Nested projects can be added manually.',
  launching: 'Launching', codePath: 'VS Code executable', autoDetect: 'Detect automatically', browse: 'Browse', reset: 'Reset', terminal: 'Terminal', terminalAuto: 'Windows Terminal, then PowerShell',
  shortcut: 'Global shortcut', shortcutHint: 'RepoJump must be running. A conflicting shortcut keeps your previous setting.', shortcutPlaceholder: 'Ctrl+Alt+P', shortcutEnabled: 'Enable global shortcut', capture: 'Record', recording: 'Press shortcut…',
  closeToTray: 'Keep running in the tray when the window is closed', localData: 'Local data', localDataHint: 'Settings and your project index stay on this device.',
  scanIssues: 'Some folders could not be scanned', details: 'Details', launchPending: 'This project is already being opened.',
  directoryUnavailable: 'This folder is missing or cannot be accessed.', invalidPath: 'Choose an existing folder using an absolute path.', rootDuplicate: 'This code root has already been added.', rootNotFound: 'This code root no longer exists in RepoJump.', projectNotFound: 'This project is no longer in the index. Rescan and try again.',
  storageFailure: 'Local data could not be saved. Check folder permissions and available disk space.', storageCorrupt: 'The local settings file is invalid.', storageReadOnly: 'Local data is read-only. Changes cannot be saved.', storageNewerVersion: 'These settings were created by a newer RepoJump. Install that version to use them safely.',
  storageRecovered: 'Settings were recovered from a backup. The damaged file has been preserved.', storageReset: 'The settings file was damaged and has been preserved. No valid backup was found; a new configuration is being used.', cacheFailure: 'The project cache could not be saved. Your settings are retained.',
  vscodeInvalid: 'The configured VS Code executable is unavailable. Change it in Settings.', vscodeNotFound: 'VS Code could not be found. Select Code.exe in Settings.', launchFailed: 'The project could not be opened.', recentSaveFailed: 'VS Code was started, but Recent could not be saved.',
  terminalNotFound: 'The selected terminal is unavailable. Choose Automatic or PowerShell in Settings.', repositoryUnavailable: 'No supported origin repository URL was found.', clipboardFailed: 'The path could not be copied.',
  shortcutConflict: 'The global shortcut could not be registered. Choose another shortcut in Settings.', shortcutInvalid: 'Enter a valid shortcut, such as Ctrl+Alt+P.', invalidSettings: 'Some settings are invalid.', categoryInvalid: 'Enter a category name with 1–64 characters.',
  gitUnavailable: 'Git information could not be read.', unexpected: 'Something went wrong. Try again.', directoryMissing: 'Folder missing', directoryUnreadable: 'Folder cannot be read',
  pickerFailed: 'The folder or file picker could not be opened.', selected: 'selected', quick: 'Quick launch', enterHint: 'open project', navigateHint: 'navigate', searchHint: 'search', version: 'Version',
} as const;

type Key = keyof typeof en;
const zh: Record<Key, string> = {
  all: '全部项目', favorites: '收藏', recent: '最近打开', categories: '分类', uncategorized: '未分类',
  search: '搜索项目…', settings: '设置', addRoot: '添加代码根目录', addProject: '添加项目', rescan: '重新扫描',
  projects: '个项目', searching: '搜索结果', scanning: '正在扫描', ready: '就绪', visited: '个目录已检查',
  welcome: '更快找到你的下一个项目。', welcomeBody: '添加存放代码的根目录，RepoJump 会自动发现其中的项目。',
  noResults: '没有匹配的项目', noResultsBody: '试试项目名称、路径、分类或技术类型。',
  noFavorites: '把常用项目放在这里', noFavoritesBody: '收藏项目，让它始终显示在列表顶部。',
  noRecent: '从这里开始下一个项目', noRecentBody: '在 VS Code 中打开的项目会自动出现在这里。',
  noProjects: '没有发现项目', noProjectsBody: '可以手动添加项目文件夹、调整扫描深度，或选择其他代码根目录。',
  desktopOnly: '请启动 RepoJump 桌面程序以访问本地项目。', loading: '正在读取项目索引…', retry: '重试',
  openCode: '在 VS Code 中打开', openTerminal: '在终端中打开', openExplorer: '在资源管理器中打开', copyPath: '复制路径', openRepository: '打开仓库网页',
  favorite: '收藏', unfavorite: '取消收藏', more: '项目操作', copied: '路径已复制', category: '分类', editCategory: '修改分类', autoCategory: '恢复自动分类',
  removeManual: '移除手动添加记录', removeManualBody: '只移除手动添加记录，不操作项目文件夹。由代码根目录发现的项目仍保留在列表中。',
  manual: '手动添加', missing: '目录已不存在', unknown: '目录状态未知', git: 'Git 仓库', branch: '分支', clean: '工作区干净', dirty: '有未提交的修改', gitUnknown: 'Git 信息暂不可用',
  lastOpened: '最近打开', never: '尚未打开', open: '打开', cancel: '取消', save: '保存', remove: '移除', close: '关闭',
  roots: '代码根目录', rootsBody: '扫描这些目录；发现项目后，不再递归扫描它的子目录。', noRoots: '尚未添加代码根目录。',
  changeRoot: '修改根目录', removeRoot: '移除根目录', removeRootBody: '从 RepoJump 移除此根目录？项目文件夹不会被修改，手动添加记录和其他根目录仍然保留。',
  appearance: '外观', theme: '主题', dark: '深色', light: '浅色', system: '跟随系统', language: '语言',
  discovery: '项目发现', scanDepth: '最大扫描深度', depthHint: '根目录为第 0 层，默认扫描至第 4 层。嵌套项目可手动添加。',
  launching: '打开方式', codePath: 'VS Code 可执行文件', autoDetect: '自动检测', browse: '选择文件', reset: '恢复自动检测', terminal: '终端', terminalAuto: '优先 Windows Terminal，其次 PowerShell',
  shortcut: '全局快捷键', shortcutHint: 'RepoJump 需要保持运行。快捷键冲突时保留之前的设置。', shortcutPlaceholder: 'Ctrl+Alt+P', shortcutEnabled: '启用全局快捷键', capture: '录制', recording: '请按快捷键…',
  closeToTray: '关闭窗口后继续在托盘中运行', localData: '本地数据', localDataHint: '设置和项目索引仅保存在此设备上。',
  scanIssues: '部分目录未能完成扫描', details: '详细信息', launchPending: '此项目正在打开，请稍候。',
  directoryUnavailable: '目录不存在或无法访问。', invalidPath: '请选择使用绝对路径的现有文件夹。', rootDuplicate: '已经添加过此代码根目录。', rootNotFound: '此代码根目录已被移除。', projectNotFound: '项目已不在索引中，请重新扫描后再试。',
  storageFailure: '无法保存本地数据，请检查目录权限和剩余磁盘空间。', storageCorrupt: '本地配置文件无效。', storageReadOnly: '本地数据为只读状态，无法保存更改。', storageNewerVersion: '配置来自更新版本的 RepoJump，请使用该版本读取数据。',
  storageRecovered: '已从备份恢复设置，损坏的文件已保留。', storageReset: '配置文件已损坏且没有有效备份，原文件已保留，目前使用新配置。', cacheFailure: '无法保存项目缓存，用户设置仍然保留。',
  vscodeInvalid: '指定的 VS Code 可执行文件不可用，请在设置中修改。', vscodeNotFound: '未找到 VS Code，请在设置中选择 Code.exe。', launchFailed: '无法打开此项目。', recentSaveFailed: 'VS Code 已启动，但最近打开记录未能保存。',
  terminalNotFound: '所选终端不可用，请在设置中选择自动或 PowerShell。', repositoryUnavailable: '未找到可识别的 origin 仓库地址。', clipboardFailed: '无法复制项目路径。',
  shortcutConflict: '无法注册全局快捷键，请在设置中选择其他组合。', shortcutInvalid: '请输入有效的快捷键，例如 Ctrl+Alt+P。', invalidSettings: '部分设置无效。', categoryInvalid: '分类名称需要包含 1–64 个字符。',
  gitUnavailable: '无法读取 Git 信息。', unexpected: '操作失败，请重试。', directoryMissing: '目录不存在', directoryUnreadable: '无法读取目录',
  pickerFailed: '无法打开目录或文件选择器。', selected: '已选择', quick: '快捷启动', enterHint: '打开项目', navigateHint: '切换选择', searchHint: '搜索', version: '版本',
};

export function locale(settings: Settings): 'zh-CN' | 'en' {
  return settings.language === 'system' ? (navigator.language.startsWith('zh') ? 'zh-CN' : 'en') : settings.language;
}
export function translator(language: 'zh-CN' | 'en') { return (key: Key) => (language === 'zh-CN' ? zh : en)[key]; }
export type Translate = ReturnType<typeof translator>;
export function errorText(error: unknown, t: Translate): string {
  const code = typeof error === 'object' && error !== null && 'code' in error ? String((error as AppError).code) : 'unexpected';
  return t(code in en ? code as Key : 'unexpected');
}
