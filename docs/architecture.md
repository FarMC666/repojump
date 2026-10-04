# RepoJump architecture

## Boundaries

The React application owns presentation, keyboard navigation, locale/theme and in-memory search. `src/api.ts` is the only desktop IPC adapter. Native functionality stays in Rust; a browser preview has no filesystem mock or demo data.

The Rust backend separates directory discovery, extensible detection rules, Git metadata, JSON storage, process launching, settings, native pickers and the application service. Tauri commands validate/forward inputs. The service coordinates sources, user metadata, revisions and scan lifecycle.

## Index lifecycle

Startup loads user state and the last index before starting a scan. The UI subscribes to `index-updated` before requesting `bootstrap`, and accepts snapshots monotonically by revision.

One scan worker operates at a time. Repeated requests set a pending flag. Changes to roots or depth increment a generation; obsolete callbacks stop publishing. Results are merged in batches, preserving user metadata. Unreadable subtrees protect corresponding cached records; a missing directory is retained with a missing status. Overlapping roots contribute separate sources to the same normalized project identity. Manual sources survive root removal.

Discovery enumerates a directory once. Detection combines marker rules and bounded manifest reads. It stops below a detected project, skips ignored directories and does not follow directory reparse points. Explicit roots and manual projects are canonicalized before registration.

The primary add action registers a code root. A manual folder selection first checks project markers; a folder without markers offers root scanning or an explicit single-folder entry. Existing manual containers can be promoted by project ID: the service deduplicates the root and removes only the manual source in one persisted mutation, retaining favorite/recent/category metadata. Root changes reset the UI to All and clear the previous query. Search normalizes Windows path separators in memory.

## State and persistence

`state.json` and `index.json` have independent schema versions. User mutations clone state, persist it, then publish the new revision. A failed write leaves the previous in-memory user state intact. Discovery only mutates cached detection fields.

Configuration writes preserve a valid backup before atomic replacement. Corrupt primary files are quarantined; a valid backup is recovered with a visible warning. A newer unsupported user schema blocks writes instead of replacing it with defaults. Cache failures are nonfatal and do not discard user settings.

Storage resolves the actual application data directory after creating it. This keeps temporary and destination paths on the same volume when a Windows packaged host redirects AppData or when a directory is a junction.

The bundle identifier `com.farmc.repojump` is also the stable data-directory identity. Keep it fixed across releases.

## Opening and desktop lifecycle

Opening accepts a registered project ID and a fixed target enum. VS Code and terminals use executable/argument arrays, with no user paths embedded in shell scripts. Windows CLI shims are resolved to the actual editor exe. A VS Code spawn records Recent; a later persistence failure is reported as a successful launch with an unsaved Recent warning, preventing retries from opening duplicate windows.

Per-project VS Code startup overrides are user metadata, separate from the rebuildable index. They default to normal opening for legacy state. File selections are canonicalized and checked against the project root both when saved and when launched, including reparse-point resolution. All launch entrypoints use the same backend configuration.

Git Graph opening installs only the bundled `farmc.repojump-startup` companion VSIX, using the configured editor's Node CLI and a 30-second process deadline. It never installs Git Graph. Current Windows installations locate `cli.js` from the editor's own `bin/code.cmd`; the shim is read as data, never executed. The CLI's Electron Node environment is scoped to that process and removed for editor window launches.

Each Graph launch creates a unique single-folder `.code-workspace` under application data. Its workspace-only setting points to a sibling request with a UUID, exact project path and 15-second expiration. The companion activates after startup, verifies that the request matches its window's only local folder, atomically claims it, activates Git Graph and invokes `git-graph.view` with the project's `rootUri`. It respects workspace trust and never accepts arbitrary commands. A matching atomic receipt reports success or a localized failure to Rust. Only transient request/receipt files are removed; workspace files remain usable for restore, without adding generated entries to VS Code's Recent list.

Startup-content failures preserve an already opened window or fall back to one ordinary project launch before any workspace is opened. `LaunchResult.warnings` reports all startup and Recent-save warnings together. Quick launch remains visible when warnings need attention.

Git is optional. Its child processes have bounded execution time, drained/bounded output, no prompts or lazy fetching, optional write locks disabled and fsmonitor disabled. Only HTTP/S repository URLs reach the OS opener; SSH conversion preserves the host and removes credentials.

Single-instance startup, tray activation and global shortcuts all reveal the same window. Only shortcut activation enters quick-launch mode. Shortcut changes register the replacement before releasing the active shortcut and roll back if persistence fails. Configured shortcuts and successfully registered shortcuts are tracked separately, so a startup conflict can be repaired.

The native picker uses the official Windows API bindings on a fresh STA worker thread. It owns and releases its COM apartment and handles cancellation explicitly, keeping user selection off the async executor. The frontend has no generic filesystem or shell permissions.

## Validation

Vitest covers search semantics and 500-project search performance. Rust tests cover markers, multi-label detection, recursion rules, identity/category behavior, literal launch arguments, origin conversion, source preservation, unavailable roots and configuration recovery/version protection. Real desktop acceptance additionally checks native selection, actual editor/terminal/Explorer launching, global key activation, tray/close behavior, restart persistence and installer upgrade behavior.

An optional `REPOJUMP_TEST_DATA` environment variable redirects only **debug builds** to an isolated user-state directory for native QA. Release builds always use the stable application data directory. Temporary QA scripts, fixtures and screenshots live under ignored `.validation/`; build/download caches live under ignored `.tools/`.

Debug builds also accept an absolute `REPOJUMP_TEST_VSCODE_PROFILE` directory. Both editor launches and extension-management CLI calls use its `user-data/` and `extensions/` children, keeping native startup QA out of the user's editor profile. This override is compiled out of release builds.
