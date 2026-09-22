# Manual Windows acceptance matrix

Record OS build, WebView2 version, display layouts and date. Each unchecked item is NOT validated by compilation.

- [ ] Start without elevation. Tray and settings visible; overlay background transparent.
- [ ] Test WorkerW mode and forced fallback mode. Normal applications cover the companion; Win+D remains usable.
- [ ] Click/double-click/drag 20 desktop icons outside sprite: no lost events.
- [ ] Hold sprite 249ms then release: no lock. Hold 250ms: lock indication.
- [ ] Set quantity to 3. Capture and drag each of the three companions separately; the pressed companion, not the first companion, must follow the pointer.
- [ ] Repeat capture in WorkerW mode and bottom-window fallback mode. A sprite over an ordinary app window or taskbar must not capture.
- [ ] Drag captured sprite: no Explorer selection rectangle or underlying icon drag.
- [ ] Target a disposable desktop file and release immediately: item moves to Recycle Bin without a dwell delay; restore it and compare contents.
- [ ] Release outside a reliably resolved desktop item: refusal and no deletion.
- [ ] Repeat with .lnk (only link recycled; destination untouched), empty folder, nonempty ordinary folder.
- [ ] Disable Recycle Bin or use oversized item: operation must refuse permanent deletion.
- [ ] Refuse This PC, Recycle Bin icon, network paths, symlinks, junctions, cloud placeholders, roots and protected/read-only items.
- [ ] Rename/replace/delete the target during a drag: refusal, no other file removed.
- [ ] Drag over app windows/taskbar/menu/empty desktop: no hidden desktop target.
- [ ] Cancel capture 20 times using Esc/right click. No stale operation or swallowed unrelated input.
- [ ] Press Win+D twice, close Settings, and close an unrelated window: companion remains on the desktop and dynamic wallpaper keeps animating.
- [ ] Explorer restart, app exit and forced process termination: desktop and mouse remain usable.
- [ ] 100/125/150/200% DPI; left-hand negative coordinate screen; primary change; disconnect monitor mid-drag: cancel, rebuild, no wrong target.
- [ ] Fullscreen app pauses corresponding screen. Alt-tab restores motion.
- [ ] Skin: valid folder and .jpskin import; switch/restart; invalid manifest, duplicate paths, traversal, symlinks, scripts, executables, ZIP bombs, oversized image/frame count: reject without partial installation.
- [ ] Run 2 hours at 30 FPS; record memory/CPU, sleep/resume, monitor changes.
- [ ] NSIS install/uninstall per-user; portable on clean Windows with WebView2; current-user startup toggle.

If capture still fails, inspect `%APPDATA%/org.junopulsar.desktop/diagnostics.log`. A successful press records `input capture accepted`; a geometric hit rejected because another window covers the desktop records `input capture surface_rejected`. The log does not contain coordinates, filenames or paths.
