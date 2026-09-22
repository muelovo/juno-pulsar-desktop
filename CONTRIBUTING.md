# Contributing

Use main for releasable code; feat/* and fix/* for changes. Keep Rust unsafe blocks documented and small; never infer target paths from visible icon text. All filesystem mutation must remain in Rust with policy checks. New skin formats require decode budgets and adversarial tests.

Run npm test, npm run build, cargo test, cargo fmt --check and cargo clippy. Run docs/manual-tests.md on Windows 10 and 11 before release. Do not commit game reference images, private paths, logs, credentials or compiled target directories. Update CHANGELOG and synchronize all version fields.
