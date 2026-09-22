# Manual Windows acceptance matrix

Record OS build, WebView2 version, display layouts and date. Each unchecked item is NOT validated by compilation.

- [ ] Start without elevation. Tray and settings visible; overlay background transparent.
- [ ] Test WorkerW mode and forced fallback mode. Normal applications cover the companion; Win+D remains usable.
- [ ] Click/double-click/drag 20 desktop icons outside sprite: no lost events.
- [ ] Hold sprite 249ms then release: no lock/confirmation. Hold 250ms: lock indication.
- [ ] Drag captured sprite: no Explorer selection rectangle or underlying icon drag.
- [ ] Target a disposable desktop file; hover 599ms and release: no confirmation.
- [ ] Hover 600ms and release: exact filename/type/path; Cancel default focus. Cancel via button/Esc/right click/title close.
- [ ] Explicitly confirm ONLY a disposable test file. Verify item in Recycle Bin, restore it and compare contents.
- [ ] Repeat with .lnk (only link recycled; destination untouched), empty folder, nonempty ordinary folder.
- [ ] Disable Recycle Bin or use oversized item: operation must refuse permanent deletion.
- [ ] Refuse This PC, Recycle Bin icon, network paths, symlinks, junctions, cloud placeholders, roots and protected/read-only items.
- [ ] Rename/replace/delete target while confirmation is open: refusal, no other file removed.
- [ ] Wait >30 seconds or replay token: refusal. Turn deletion off with confirmation open: refusal.
- [ ] Drag over app windows/taskbar/menu/empty desktop: no hidden desktop target.
- [ ] Cancel capture 20 times using Esc/right click. No stale confirmation or swallowed unrelated input.
- [ ] Explorer restart, app exit and forced process termination: desktop and mouse remain usable.
- [ ] 100/125/150/200% DPI; left-hand negative coordinate screen; primary change; disconnect monitor mid-drag: cancel, rebuild, no wrong target.
- [ ] Fullscreen app pauses corresponding screen. Alt-tab restores motion.
- [ ] Skin: valid folder and .jpskin import; switch/restart; invalid manifest, duplicate paths, traversal, symlinks, scripts, executables, ZIP bombs, oversized image/frame count: reject without partial installation.
- [ ] Run 2 hours at 30 FPS; record memory/CPU, sleep/resume, monitor changes.
- [ ] NSIS install/uninstall per-user; portable on clean Windows with WebView2; current-user startup toggle.
