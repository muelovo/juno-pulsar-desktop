# Native implementation references

- Tauri window configuration: https://v2.tauri.app/reference/config/
- Desktop Shell view/PIDL pattern: https://devblogs.microsoft.com/oldnewthing/20130318-00/?p=4933
- IFileOperation flags: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags
- PreDeleteItem callback: https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-predeleteitem

Implementation uses a dedicated STA per Shell operation; no COM interface crosses threads. On the tested Explorer, querying an additional IOleWindow interface returned E_NOINTERFACE. Calling IShellView's inherited GetWindow method succeeds.

2026-09-22 local read-only diagnostic: 37 desktop items, 36 filesystem-path items, SysListView32 found. No filenames/paths emitted. Native 8-second startup/exit smoke test exited normally; WorkerW discovery fell back to bottom mode on this host. This is not a visual/real-delete acceptance test.
