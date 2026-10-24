# Changelog

All notable changes to Parch Linux Gamehub will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-24

### Added

- **Game Library & Proton Runner**:
  - Full game management library to add, view, edit, and organize Windows and custom games.
  - Dedicated Games page with responsive card grid view and quick-action run/kill buttons.
  - Game Details pane displaying playtime, Proton version, prefix path, launch options, and game cover.
  - Add/Edit Game dialog with automatic executable discovery and heuristic candidate ranking.
  - Steam match detection with automatic artwork (header/capsule/icon) and AppID fetching from Steam Store API.
  - Direct Winetricks integration for per-prefix component and DLL management.
  - Process monitoring and one-click game termination support.
- **Proton & Gaming optimizations**:
  - `umu-launcher` integration with automatic fallback to system Proton.
  - GameMode integration for low-latency gaming execution.
  - Custom environment variables, command arguments, and prefix architecture isolation.

### Packaging

- Added `umu-launcher`, `gamemode`, and `winetricks` as optional dependencies in `PKGBUILD`.
- Added `git` to `makedepends` in `PKGBUILD`.

## [0.1.1] - 2026-07-21

### Fixed

- **Application not exiting on window close**: Closing the window with the X button now properly terminates the process. Previously, the process continued running in the background consuming CPU/memory due to a missing `app.quit` action and no `close_request` handler on the window.
- **Proton page never loading**: The Proton compatibility tools page would spin forever and never display content. The idle callback was resetting its results accumulator on every tick, discarding previously received GitHub API responses.
- **Proton refresh button**: Fixed the same accumulator bug in the refresh handler, and removed a double `try_recv()` call that silently consumed and discarded results.
- **Launcher locale leak**: Strip `LANGUAGE` fallbacks before spawning child processes to prevent locale issues. (commit 2bbd784)
- **PKGBUILD dependency**: Fixed `rsvg-convert` dependency - it's part of `librsvg`. (commit 2e03618)
- **Proton-CachyOS RTSP collision**: Added `match_fn` to `CompatTool` to disambiguate installed version detection. Previously `GE-Proton-RTSP` was incorrectly listed under both `GE-Proton` and `GE-Proton-RTSP`. (commit f78be4d)
- **Proton-CachyOS no releases showing**: Two bugs: (1) `asset_filter` regex `r"proton-cachyos.*\.tar\.zst"` didn't match any CachyOS releases - they use `.tar.xz` format. Changed to `r"proton-cachyos.*\.tar\.(?:xz|zst)"`. (2) `arch_filter: Some("x86_64")` matched all arch variants (`x86_64`, `x86_64_v2`, `x86_64_v3`, `x86_64_v4`) via substring, usually picking the experimental `x86_64_v4` asset. Changed to `Some("-x86_64.tar")` to only match the standard variant.
- **curl error handling**: Removed `-f` flag so HTTP errors (4xx/5xx, rate limits) are captured in the response body instead of silently failing, with dedicated rate-limit error messaging.
- **PKGBUILD tag reference**: Fixed hardcoded `v0.1` tag - now interpolates `$pkgver` so building from source matches the release version.

### Changed

- **Proton page revamped to follow GNOME HIG**: Complete redesign of the Proton/Compatibility Tools page using proper GNOME Human Interface Guidelines patterns:
  - `AdwStatusPage` for loading state with descriptive text
  - `AdwComboRow` for tool selection (replaces raw dropdown in ActionRow)
  - Tool description row with info icon that updates on tool selection
  - `AdwExpanderRow` for release notes (replaces raw TextView)
  - Proper `boxed-list` styled ListBoxes for available and installed versions
  - Per-row trash button for removing individual installed versions
  - Pill-style action buttons with proper alignment
  - `AdwToastOverlay` for user feedback (refresh complete, no selection, etc.)
  - Refresh button as a flat icon in the group header suffix
  - Proper icon prefixes on all rows (package, folder, checkmark, warning)
  - Install command now respects custom Steam library paths from `ProtonManager::steam_install_dirs()` and auto-detects tarball compression format (zst, xz, gz).
- **Dashboard polling**: Switched from `idle_add_local` to `timeout_add_local(Duration::from_millis(100))` for status updates. Reduces unnecessary CPU wakeups when no data is available, yielding ~3 FPS improvement.
- **Settings page**: CPU governor, kernel parameters, and esync/fsync switches now execute their respective system commands via the terminal dialog with `pkexec` elevation instead of just persisting the config toggle.

### Added

- `Ctrl+Q` keyboard shortcut now properly quits the application (action was previously registered but never created)
- **`apply_locale()` utility**: New helper in `launcher_manager` that sanitises `LANG`, `LANGUAGE`, and `LC_MESSAGES` before spawning child processes. Applied to launcher launches, dashboard quick-launch, terminal operations, and tool page invocations.
- **Minigalaxy detection**: Added to the installed launchers list in system info.
- **Per-row installed version removal**: Each installed version row has a trash button that opens a terminal dialog to `rm -rf` the install directory.

## [0.1.0] - 2025-07-14

### Added

- Initial release of Parch Linux Gamehub
- Dashboard with system status (GPU, drivers, Vulkan, Chaotic AUR)
- Launcher management (Steam, Lutris, Heroic, Bottles, Minigalaxy)
- Proton/Wine compatibility tool management (GE-Proton, Proton-CachyOS, DXVK, VKD3D-Proton)
- Steam Session management
- Gaming tools installation (Wine, performance tools, Chaotic AUR)
- Settings page for gaming optimizations
- Terminal dialog for package operations
- Icon caching for launcher icons
- Welcome banner with dismissal persistence via GSettings
