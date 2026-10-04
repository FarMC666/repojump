# RepoJump

[English](README.md) | [简体中文](README.zh-CN.md)

A lightweight Windows launcher for local development projects. Add a code root, find a project, and press **Enter** to open it in a new VS Code window.

Built with **Tauri 2, React, TypeScript and Vite**. Targets Windows 10/11 x64, runs locally, and needs no account.

## Getting started

1. Install and launch RepoJump.
2. Select **Add code root** and choose a folder such as `D:\code`.
3. RepoJump scans in the background. Search by project name, path, category or technology.
4. Use the arrow keys to select a project, then press **Enter**.

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

Choosing `D:\code` traverses `apps`, `web` and `mods` to find their projects. These container folders also supply automatic categories. Adding or changing a root switches back to All projects and clears the previous query.

Use **Add single project** for projects outside your roots, ordinary folders without project markers, or nested monorepo projects. It adds only the selected folder. If no markers are found, RepoJump offers to scan the folder as a root instead. An existing manual entry can also be converted through **Scan projects inside** in its menu, preserving favorites, recent history and category overrides.

## Features

- Instant local search, including fuzzy name matching and multiple search terms. Windows paths work with either `\` or `/`.
- Favorites pinned above other projects; Recent maintained independently of VS Code.
- Project menus for VS Code, Terminal, Explorer, copying paths, repository links and category overrides.
- Background discovery with a cached index, configurable depth and manual rescanning.
- Configurable global shortcut, tray menu and single-instance activation.
- Dark, light and system themes; English and Simplified Chinese.
- Local JSON storage with backups and a configurable data location.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+K | Focus search and select the current query |
| Up / Down | Select a project |
| Enter | Open a new VS Code window |
| Esc | Close a menu/dialog, clear the query, or hide the quick-launch window |
| Ctrl+Alt+P | Show RepoJump, clear the query and focus search |

The global shortcut can be changed or disabled in Settings. A conflict keeps the previous setting and shows an error. RepoJump must be running to receive the shortcut.

Closing the window keeps the application in the tray by default. Exit through the tray menu, or disable this behavior in Settings. A successful VS Code launch hides a window opened through the global shortcut; a normally opened window stays visible.

The tray menu follows the application's language, including Follow system, and updates when you save a language change.

## Project discovery

Recognized markers include:

```text
.git (directory or worktree file)
package.json       pnpm-workspace.yaml   yarn.lock       package-lock.json
pyproject.toml     requirements.txt      Cargo.toml      go.mod
*.sln              *.slnx                *.csproj        *.code-workspace
pom.xml            build.gradle         build.gradle.kts
composer.json      Gemfile
```

Projects can carry multiple labels, such as JavaScript, TypeScript, React and Vite. Other detectors cover Next.js, Python, Rust, Go, .NET, Maven/Gradle, PHP/Composer and Ruby. A damaged manifest does not block discovery.

The default maximum depth is **4**, with the root at depth **0**. Settings allows 1–8. **Discovery stops below a detected project**; add nested projects manually. Symbolic links and junctions are not traversed. Generated directories such as `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, virtual environments, `vendor` and `.repojump` are skipped.

Categories come from the first container folder below the most specific matching root. Direct children of a root, the root itself, and manual projects outside roots are Uncategorized. Overlapping roots and manual sources do not duplicate projects.

Startup displays the cache before refreshing it. There is no filesystem watcher: use **Rescan** after creating projects or changing markers. Unreadable roots retain cached entries, and missing folders are marked unavailable. Removing a root or manual entry only changes the index sources.

## Opening projects

**VS Code:** RepoJump launches `Code.exe` with `--new-window` and the project path as separate arguments. It checks the configured executable first, then PATH, registry entries and common installation locations. Set a path in Settings if automatic detection fails. Spaces, Chinese characters and shell punctuation remain part of the path.

Use **VS Code startup content** in the project menu to save one option per project. Enter, double-click and the Open button all apply it:

- **Default:** use VS Code's normal startup behavior.
- **File:** enter a project-relative path, such as `index.html`, or use the native file picker. The file must exist inside the project and becomes the active editor when launched.
- **Git Graph:** show Git Graph in the new project window. Install and enable `mhutchie.git-graph` first; RepoJump automatically installs its bundled companion extension on first use. This mode opens a unique workspace stored in application data without changing project files. Only the current RepoJump request triggers Git Graph; manually opening a project or restoring a consumed workspace does not repeat it.

An unavailable file, installation failure, missing Git Graph or timeout still leaves the project open and shows a warning. The saved option remains, and Recent is updated. Selecting Default removes the project's override.

**Terminal:** Automatic mode tries Windows Terminal, then PowerShell. PowerShell uses the process working directory and `-NoProfile -NoExit`; project paths are never inserted into a shell script. Paths containing semicolons use PowerShell to avoid Windows Terminal command-separator ambiguity.

**Repository:** The project menu reads Git's `origin` and converts supported HTTP/S, SSH and scp-style URLs to browser links while preserving the host. GitHub, GitLab and other hosts are supported; Azure DevOps SSH URLs have a dedicated conversion.

Git metadata is optional and read on demand with a short cache and process timeout. Git failures do not prevent discovery or launching. RepoJump does not install dependencies, change branches, or run pull, commit or push commands. It only writes its own application data.

## Local data

By default, RepoJump creates a **hidden `.repojump` folder in the first code root you added**:

```text
D:\code\.repojump\
├── state.json
├── state.json.bak
└── index.json
```

With no root, or when that location is unavailable, the fallback is:

```text
%LOCALAPPDATA%\com.farmc.repojump\.repojump
```

Later roots do not change the automatic location. Removing the first root selects the next root, or the fallback if none remain. Changing the first root transfers the data to its new path.

In **Settings → Local data**, choose **Custom folder**, select its parent directory, and save. RepoJump creates `.repojump` inside that directory and transfers your roots, manual entries, favorites, recent history, settings, category overrides and cache. **Current data folder** shows the actual active path. Returning to Automatic uses the first root again. Canceling Settings leaves the location unchanged.

- `state.json` holds user state, including per-project VS Code startup overrides, with a schema version and profile identity.
- `state.json.bak` retains the previous valid state.
- `index.json` is a rebuildable project cache.

The original `%LOCALAPPDATA%\com.farmc.repojump` directory keeps a small `storage-location.json` locator and a `.repojump` recovery copy. This lets RepoJump start when a code drive is disconnected. Changes made during fallback are retained when the configured drive returns and RepoJump restarts. WebView2's own runtime data remains in the system application-data location.

Git Graph workspaces remain under `vscode-launches/` in that original application-data directory even when preferences move elsewhere. Requests and receipts are cleaned after consumption or expiry; workspace files remain for window restoration and are excluded from VS Code's recently opened list.

Migration writes the new data before switching the locator and retains the old copy. A destination belonging to another profile is rejected. Existing AppData-only installations are migrated automatically. Configuration files use atomic replacement; damaged files are preserved and valid backups are recovered with a visible warning. Unsupported newer schemas are protected from writes. Installer upgrades retain application data.

If separate startup environments migrated the same configuration twice, Automatic storage reuses the existing root data when the user records match. Different records and custom-location profile conflicts remain protected. A fallback warning appears only while the preferred location is actually unavailable.

The `.repojump` directory has the Windows Hidden attribute. Enable **Hidden items** in Explorer to inspect it. If a code root is itself a Git repository, add `.repojump/` to your own ignore rules to keep local preferences out of Git; RepoJump does not edit project `.gitignore` files.

## Development

Requirements: Node.js 22.12+, Rust stable with the MSVC toolchain, Microsoft C++ Build Tools with **Desktop development with C++** and a Windows SDK, and WebView2. See the [official Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
npm ci
npm run desktop
```

`npm run dev` starts only the browser frontend. Local project access requires the desktop application; the browser view does not simulate native functionality.

```powershell
npm run typecheck
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run package
```

The Windows x64 NSIS installer is generated in `src-tauri/target/release/bundle/nsis/`. Packaging tools are cached in `src-tauri/target/.tauri/`. Installation is per user and does not require administrator privileges. If WebView2 is missing, the installer downloads its bootstrapper; that step requires internet access. The core application runs offline after installation. Builds are unsigned unless you supply signing credentials. See [Tauri's Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/).

See [architecture](docs/architecture.md) for module boundaries and [validation records](docs/validation.md) for the checks and platforms actually exercised.

The companion extension lives in `vscode-helper/`. `npm run build:helper` generates `src-tauri/resources/repojump-vscode.vsix` using Node.js built-in libraries. Desktop development, production builds and direct Cargo builds generate the resource; the installer bundles it for offline installation.
