# Baseline evidence — 2026-11-03

Reference commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd` verified and checked out detached. The malformed reference identifier in the plan header is superseded by `reference/source.json`.

PHP 8.5.11, Composer, Node 26.3.0 and a workspace-local MySQL 8.4.7 are available. Original locked PHP dependencies, game data and client assets are materialized in the ignored reference checkout. The original schema and catalogs were migrated/seeded in isolated `ninja_reference`; Rust uses `ninja_rust_test`, with `ninja_ci_test` reserved for synthetic CI checks. Docker is unavailable locally. No upstream user credentials or private keys were reused.

The actual AIR client runs against Rust using a separate local copy with loopback endpoint constants. The user confirmed battle and completion of the first mission. The full original Laravel/realtime stack has not been played for differential comparison.

What was executed:

- 388 first-party PHP and 18 realtime JavaScript syntax checks: all pass; see `baseline-static-checks.md`.
- Standalone original SabreAMF serializer and original diagnostic VersionService: generated seven golden binary files (three codec fixtures, four gateway request/response fixtures).
- Rust codec/gateway, authentication and socket tests: 28 regular tests; reference byte equality covers the diagnostic method. Expanded AMF/MySQL integration covers accounts, progression, shops, exams, companions, crafting, talents and Sage Mode with original and minimal fixture catalogs.

The `.tools/` Rust 1.93.0 GNU installation and portable w64devkit 2.5.0 are workspace-local and ignored by Git. No machine PATH setting was changed. `scripts/dev-env.ps1` activates them for the current shell only.

Remaining baseline work: original-server gameplay comparison, additional actual AIR acceptance and sanitized realtime packet captures. Headless gateway/database tests establish wire shapes and durable behavior; they do not verify rendered combat or every client panel. See [progress](migration/progress.md) and [local setup](local-development.md).
