# Changelog

## [0.2.0] - 2026-09-23

- Attach the overlay to the actual Progman/WorkerW desktop host so Win+D and closing windows do not minimize the companion or pause dynamic wallpaper.
- Remove the 600ms target dwell and confirmation window from drag deletion; release over an exact Shell target now moves it directly to the Recycle Bin.
- Make locked movement track the pointer without spring lag and poll desktop targets at interactive speed.
- Add a new original generated pulse-drone skin, phase effects, refined status pill, procedural lock/launch sounds, orbital flight paths, and support for up to ten companions.

All notable changes follow SemVer. Version 0.x is an evolving development preview.

## [0.1.1] - 2026-09-23

### Fixed
- Register independent native hit regions for every visible companion, so quantity 2/3 remains interactive.
- Return the captured companion index from the native hook and keep that exact companion attached during lock and drag.
- Accept the app's click-through overlay and current Explorer desktop surface in bottom-window fallback mode while still rejecting ordinary Explorer windows and the taskbar.
- Record path-free counters when a sprite press is accepted or rejected by desktop-surface validation.

### Validation
- Added a native three-region selection test; 21 Rust tests and 6 frontend state tests pass.
- Real non-injected pointer behavior still requires manual verification on each Windows desktop configuration.

## [0.1.0] - 2026-09-22

### Added
- Tauri 2 transparent WorkerW layer and bottom-window fallback.
- Original Canvas companion, timed capture/hover, Chinese settings and system tray.
- PIDL-based desktop target resolution and guarded IFileOperation recycle confirmation.
- Per-user configuration/autostart, monitor selection and DPI coordinate conversion.
- Strict local PNG skin folder/.jpskin import with resource budgets.
- Unit tests, manual acceptance checklist, NSIS/portable build and draft Release workflow.

### Limitations
- Desktop integration, recycle/restore, mixed DPI and long-running performance need manual acceptance.
- Audio and animated WebP/APNG are not included. Extra companions are visual only.
- Unsigned development builds; no telemetry or automatic update network requests.
