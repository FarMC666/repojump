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

Storage resolves the actual data directory after creating it. The locator's parent is anchored to the newly created recovery folder's actual parent, since a packaged host can redirect new writes while reads of an existing AppData directory still resolve to the original volume. This keeps temporary and destination paths on the same volume. Legacy state is read from its original logical path before migration.

The bundle identifier `com.farmc.repojump` is also the stable data-directory identity. Keep it fixed across releases.

`data_location::StorageManager` owns the active store inside the service mutex, so relocation and scan writes cannot race. Automatic storage uses `.repojump` under the first root in insertion order, or under the default local app-data directory when there are no accessible roots. A custom setting selects its parent directory; Windows marks the child folder hidden, and discovery ignores it.

The original local app-data directory retains `storage-location.json` and a `.repojump` recovery copy. Upgrades read the legacy AppData state before moving it. A relocation checks the destination profile ID, writes state and cache, then atomically replaces the locator before changing the active store. Old copies remain; another profile or an invalid destination is never silently overwritten. Primary-save success and recovery-copy failure are reported separately.

An offline location falls back to the recovery copy and updates the locator. The next startup retries the configured location using the latest fallback state, preventing a reconnected drive's stale copy from reverting preferences. User mutations also fall back if the active drive disappears while the application is running. Unsupported schema versions remain read-only.

## Opening and desktop lifecycle

Opening accepts a registered project ID and a fixed target enum. VS Code and terminals use executable/argument arrays, with no user paths embedded in shell scripts. Windows CLI shims are resolved to the actual editor exe. A VS Code spawn records Recent; a later persistence failure is reported as a successful launch with an unsaved Recent warning, preventing retries from opening duplicate windows.

Git is optional. Its child processes have bounded execution time, drained/bounded output, no prompts or lazy fetching, optional write locks disabled and fsmonitor disabled. Only HTTP/S repository URLs reach the OS opener; SSH conversion preserves the host and removes credentials.

Single-instance startup, tray activation and global shortcuts all reveal the same window. Only shortcut activation enters quick-launch mode. Shortcut changes register the replacement before releasing the active shortcut and roll back if persistence fails. Configured shortcuts and successfully registered shortcuts are tracked separately, so a startup conflict can be repaired.

The native picker uses the official Windows API bindings on a fresh STA worker thread. It owns and releases its COM apartment and handles cancellation explicitly, keeping user selection off the async executor. The frontend has no generic filesystem or shell permissions.

## Validation

Vitest covers search semantics and 500-project search performance. Rust tests cover markers, multi-label detection, recursion rules, identity/category behavior, literal launch arguments, origin conversion, source preservation, unavailable roots and configuration recovery/version protection. Real desktop acceptance additionally checks native selection, actual editor/terminal/Explorer launching, global key activation, tray/close behavior, restart persistence and installer upgrade behavior.

An optional `REPOJUMP_TEST_DATA` environment variable redirects only **debug builds** to an isolated user-state directory for native QA. Release builds always use the stable application data directory. Temporary QA scripts, fixtures and screenshots live under ignored `.validation/`; build/download caches live under ignored `.tools/`.
