# Security

Do not attach private filenames or full paths to public issues. Send minimal synthetic reproductions. No telemetry is collected. Delete proposals are local, expiring and single-use; the confirmation window is the only permitted caller of the recycle command.

Threats and limits are documented in docs/architecture.md and README.md. Treat desktop Shell behavior and arbitrary same-user filesystem mutation as untrusted. Unresolved targets fail closed. No permanent-delete API is used and elevation is never requested.

For releases, enable GitHub private vulnerability reporting before publishing. Until a repository is configured, no external reporting destination is provided.
