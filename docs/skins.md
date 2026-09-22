# Skin format v1

Import a flat folder or `.jpskin` (ZIP) containing `manifest.json`, `preview.png`, `idle.png`, `locked.png`, `target.png`, `hit.png`, `trail.png`. ZIP entries must be at archive root (no wrapper folder). This release intentionally accepts PNG **horizontal sprite sheets**, not animated WebP/APNG, arbitrary nested paths, SVG, scripts or audio. Audio is not implemented.

Each animation declares file, frames (1–120), fps (1–60). Frames are equally sized cells from left to right; image width must divide by frame count. Rendering is centered on the frame. Keep visible sprite within a 60px square at assetScale 1 for accurate capture; assetScale scales visual artwork. Collision is a conservative circular 30px core and never inferred from imported scripts.

Maximum 32 files, 8 MiB/file, 16 MiB compressed archive, 32 MiB uncompressed total, 4096px per image dimension, 64 MiB decoded pixels. Windows reserved names, colon/ADS, backslashes, slash, dot segments, case-colliding ZIP entries, symlinks and extra files rejected. JSON unknown fields rejected. Reimporting an installed ID is rejected. New skin ID required for updates in v1. Import validates all bytes before committing a unique staging directory. Loading revalidates installed assets. Data never leaves the machine.

Original sample skin is CC0. No provided game images are in the sample or package. Use `Compress-Archive -Path assets/sample-skin/* -DestinationPath sample.zip`, rename resulting ZIP to `.jpskin`.
