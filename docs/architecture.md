# Architecture / implementation contract

React renders local Canvas and Chinese controls. Rust owns all filesystem operations, native input, Shell identity and policy. No command accepts a deletion path. Each Shell operation owns its interfaces within a dedicated STA. Desktop HWNDs are borrowed; only our windows may be reparented or destroyed. WorkerW discovery is undocumented and optional. A dedicated RAII mouse hook consumes only fresh sprite presses over desktop windows; position polling installs no additional hooks. Idle window is click-through. Coordinates crossing IPC are physical pixels with explicit origin and scale.

M0: compile + transparent layer + curve + input + PIDL hit probe. M1: timed lock, hover, confirmation tokens, recycle, settings/tray and tests. M2: strict skin import, monitor/DPI lifecycle. M3: reproducible build, NSIS/portable, versioned draft releases.

Trust boundaries: untrusted webview arguments; untrusted archive bytes; mutable Explorer desktop; filesystem reparse points; concurrent rename/replacement. Refuse ambiguous hit, virtual shell item, nonlocal paths, directories outside desktop children, reparse entries, protected locations, identity mismatch, expired tokens. Never resolve .lnk to its destination. Confirmation is mandatory. No permanent fallback; no elevation.

No network or telemetry. Logs exclude names/paths and only contain operation codes. Existing root game images are local references excluded by .gitignore; no packaged source art. Original procedural placeholder uses white housing, cyan energy and orange fins, without copied geometry. Audio is not included.

Tests must distinguish compile/unit checks from actual Windows Explorer integration. Publishing requires a configured GitHub repository; CI creates a draft release on SemVer tags. Do not claim a public Release exists merely because workflow files exist.

