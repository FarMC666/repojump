# RepoJump architecture

## Boundaries

The React application owns presentation, keyboard navigation, locale/theme and in-memory search. `src/api.ts` is the only desktop IPC adapter. Native functionality stays in Rust; a browser preview has no filesystem mock or demo data.

The Rust backend separates directory discovery, extensible detection rules, Git metadata, JSON storage, process launching, settings, native pickers and the application service. Tauri commands validate/forward inputs. The service coordinates sources, user metadata, revisions and scan lifecycle.

## Index lifecycle

Startup loads user state and the last index before starting a scan. The UI subscribes to `index-updated` before requesting `bootstrap`, and accepts snapshots monotonically by revision.

One scan worker operates at a time. Requests enter `indexer::WorkQueue`: a full scan supersedes pending scopes, while incremental paths are coalesced by root and directory. Work arriving during a scan remains queued for its next pass. Changes to roots or depth increment a generation; obsolete callbacks stop publishing. Results are merged in batches, preserving user metadata. Unreadable subtrees protect corresponding cached records; a missing directory is retained with a missing status. Overlapping roots contribute separate sources to the same normalized project identity. Manual sources survive root removal.

Discovery enumerates a directory once. Detection combines marker rules and bounded manifest reads. It stops below a detected project, skips ignored directories and does not follow directory reparse points. Explicit roots and manual projects are canonicalized before registration.

The primary add action registers a code root. A manual folder selection first checks project markers; a folder without markers offers root scanning or an explicit single-folder entry. Existing manual containers can be promoted by project ID: the service deduplicates the root and removes only the manual source in one persisted mutation, retaining favorite/recent/category metadata. Root changes reset the UI to All and clear the previous query. Search normalizes Windows path separators in memory.

## Filesystem monitoring

`watcher.rs` owns notify 8.2, an event thread and native handle cleanup. On Windows, opening handles for both a directory and its descendants prevents ancestor renames. RepoJump therefore maintains a single native subtree handle for each disjoint root/manual source, consolidating overlapping sources. A shared set of safely visited discovery directories filters events before they enter the bounded 2,048-event queue. Ignored directories and children of detected projects never enter that set, except explicit manual sources. The watcher does not enumerate or read symlink/junction targets; discovery revalidates every path component before reading.

Marker and tag-config events, directory creation/removal and renames become hints, debounced for 300 ms with a 2-second maximum delay. `service::request_changes` resolves hints to their root scope and existing project boundary. `indexer.rs` runs the existing discovery detector for that scope, removes only its root contribution and preserves manual sources and user metadata. Missing records retain the established missing/unknown semantics. Manual entries outside roots receive independent detection refreshes.

Full and incremental work share one worker, epoch cancellation and monotonically increasing snapshot revisions. Registration occurs before a directory read, so changes during scanning queue another pass. Source changes reset subscriptions with the generation. Cache saves skip unchanged incremental results and coalesce queued work; relocation and persistence still serialize through the service mutex. Cache writes attempt the local recovery copy even when the active drive is unavailable.

Watcher creation/registration failures are nonfatal. Health checks run about every 30 seconds, invalidate unavailable handles, retry missing registrations and reconcile UNC roots periodically. Queue overflow or native rescan requests fall back to discovery reconciliation. Exit stops the watcher and drops native handles; hiding the window leaves indexing active. Manual Rescan remains the trusted full baseline.

## State and persistence

`state.json` is schema 2; `index.json` and the storage locator remain schema 1. Schema 0/1 migration maps `settings.vscodePath` to `settings.editorProfiles.vscode.executablePath`, supplies `defaultEditorId: vscode` and preserves existing startup fields and all other user records. `editorOverrides` is a separate project-ID map. Unknown profile IDs are retained and reported as unsupported, rather than treating the entire state as corrupt. Before a valid legacy state can be replaced or relocated, its original bytes are atomically preserved in `state.pre-v2.json`; failure leaves storage read-only. Downgrades require restoring a consistent pre-migration active/recovery state and locator. User mutations clone state, persist it, then publish the new revision. A failed write leaves the previous in-memory user state intact. Discovery only mutates cached detection fields.

Configuration writes preserve a valid backup before atomic replacement. Corrupt primary files are quarantined; a valid backup is recovered with a visible warning. A newer unsupported user schema blocks writes instead of replacing it with defaults. Cache failures are nonfatal and do not discard user settings.

Storage resolves the actual data directory after creating it. The locator's parent is anchored to the newly created recovery folder's actual parent, since a packaged host can redirect new writes while reads of an existing AppData directory still resolve to the original volume. This keeps temporary and destination paths on the same volume. Legacy state is read from its original logical path before migration.

The bundle identifier `com.farmc.repojump` is also the stable data-directory identity. Keep it fixed across releases.

`data_location::StorageManager` owns the active store inside the service mutex, so relocation and scan writes cannot race. Automatic storage uses `.repojump` under the first root in insertion order, or under the default local app-data directory when there are no accessible roots. A custom setting selects its parent directory; Windows marks the child folder hidden, and discovery ignores it.

The original local app-data directory retains `storage-location.json` and a `.repojump` recovery copy. Upgrades read the legacy AppData state before moving it. A relocation checks the destination profile ID, writes state and cache, then atomically replaces the locator before changing the active store. Old copies remain; another profile or an invalid destination is never silently overwritten. Primary-save success and recovery-copy failure are reported separately.

An offline location falls back to the recovery copy and updates the locator. The next startup retries the configured location using the latest fallback state, preventing a reconnected drive's stale copy from reverting preferences. User mutations also fall back if the active drive disappears while the application is running. Unsupported schema versions remain read-only.

## Editor profiles and project actions

`editors.rs` contains the fixed editor registry, capability flags, saved profile configuration and runtime availability. `editor_detection.rs` resolves manual executable → PATH shim/exe → App Paths registry → matching Windows uninstall records → common per-user/system installs. Both registry views are read; only known product display names and existing executables with the expected basename are accepted. InstallLocation and DisplayIcon are treated as literal paths, never commands. CLI shims in `bin` or `resources/app/bin` are mapped to executables without executing or parsing the shim. Profiles contain an executable path only: no user command templates or arbitrary arguments.

Project editor overrides inherit `settings.defaultEditorId` when absent. Open With passes a configured registry ID for one launch without changing either setting. Only VS Code enables RepoJump helper and Git Graph; Insiders also enables specific-file opening; Cursor/Windsurf initially enable normal opening only. Unsupported startup content falls back to ordinary opening and preserves the saved setting. React presents this expected capability degradation as information and allows quick-mode hiding; other launch, helper and Recent warnings remain errors that keep the window visible. Settings validation rejects newly invalid executables while allowing unrelated preference changes when a previously saved editor is unavailable.

React `projectActions.ts` owns action labels, availability and typed intents. `useProjectActions.ts` executes against the latest registered project ID, handles launch de-duplication, Recent snapshots, warnings and quick-mode hiding. Menus, row buttons, Enter/double-click and `ActionPalette.tsx` use this dispatcher. Repository actions require current metadata; missing projects disable native launches while copy and writable favorites remain available. Tab opens the palette, arrows choose enabled actions, Enter runs and Esc restores the previous query/selection/caret. Ctrl+Enter and Alt+Enter map to the same terminal/Explorer actions. Dialog and IME guards preserve native form navigation.

## Opening and desktop lifecycle

Opening accepts a registered project ID and a fixed target enum. Editor and PowerShell launches use executable/argument arrays, with project directories supplied as arguments or the process working directory. Windows CMD uses `CreateProcessW` with separate executable and working-directory fields, a fixed `cmd.exe /D /K` command line and a new console that does not inherit application stdio handles. Windows Terminal uses its default shell; CMD is an explicit, independent setting. User paths never enter shell scripts. Windows CLI shims are resolved to the actual editor exe. An editor spawn records Recent; a later persistence failure is reported as a successful launch with an unsaved Recent warning, preventing retries from opening duplicate windows.

VS Code startup defaults live in Settings and per-project overrides are user metadata, separate from the rebuildable index. Legacy settings default to normal opening. Projects without an override inherit the global default; changing the global default preserves existing overrides. Global file paths are validated as relative paths when saved, then resolved and checked inside each launched project. Project-specific file selections are canonicalized and checked against the project root both when saved and when launched, including reparse-point resolution. All launch entrypoints use the same backend configuration.

Git Graph opening installs only the bundled `farmc.repojump-startup` companion VSIX, using the resolved VS Code profile's Node CLI and a 30-second process deadline. It never installs Git Graph. Current Windows installations locate `cli.js` from the editor's own `bin/code.cmd`; the shim is read as data, never executed. The CLI's Electron Node environment is scoped to that process and removed for editor window launches.

Each Graph launch creates a unique single-folder `.code-workspace` under application data. Its workspace-only setting points to a sibling request with a UUID, exact project path and 15-second expiration. The companion activates after startup, verifies that the request matches its window's only local folder, atomically claims it, activates Git Graph and invokes `git-graph.view` with the project's `rootUri`. It respects workspace trust and never accepts arbitrary commands. A matching atomic receipt reports success or a localized failure to Rust. Only transient request/receipt files are removed; workspace files remain usable for restore, without adding generated entries to VS Code's Recent list.

The launch directory uses `StorageManager`'s resolved bootstrap path and stays fixed when the active preference store moves. Startup overrides are transferred and mirrored with other user metadata. Duplicate-profile recovery checks the original user records and combines nonconflicting startup and editor-override additions; incompatible options for the same project remain protected. Legacy configurations without the new field do not cause a false fallback.

Startup-content failures preserve an already opened window or fall back to one ordinary project launch before any workspace is opened. `LaunchResult.warnings` reports all startup and Recent-save warnings together. Quick launch remains visible when warnings need attention.

Git is optional. Its child processes have bounded execution time, drained/bounded output, no prompts or lazy fetching, optional write locks disabled and fsmonitor disabled. Only HTTP/S repository URLs reach the OS opener; SSH conversion preserves the host and removes credentials.

Single-instance startup, tray activation and global shortcuts all reveal the same window. Only shortcut activation enters quick-launch mode. Shortcut changes register the replacement before releasing the active shortcut and roll back if persistence fails. Configured shortcuts and successfully registered shortcuts are tracked separately, so a startup conflict can be repaired.

The native picker uses the official Windows API bindings on a fresh STA worker thread. It owns and releases its COM apartment and handles cancellation explicitly, keeping user selection off the async executor. The frontend has no generic filesystem or shell permissions.

## Validation

Vitest covers search semantics, 500-project search performance, action availability and rendered keyboard/palette/quick-mode flows using jsdom. Rust tests cover markers, multi-label detection, recursion rules, identity/category behavior, literal launch arguments, origin conversion, source preservation, unavailable roots and configuration recovery/version protection. Real desktop acceptance additionally checks native selection, actual editor/terminal/Explorer launching, global key activation, tray/close behavior, restart persistence and installer upgrade behavior.

An optional `REPOJUMP_TEST_DATA` environment variable redirects only **debug builds** to an isolated user-state directory for native QA. Release builds always use the stable application data directory. Temporary QA scripts, fixtures and screenshots live under ignored `.validation/`; build/download caches live under ignored `.tools/`.

Debug builds also accept an absolute `REPOJUMP_TEST_VSCODE_PROFILE` directory. Editor launches and VS Code extension-management CLI calls use its `user-data/` and `extensions/` children, keeping native startup QA out of the user's editor profile. This override is compiled out of release builds.
