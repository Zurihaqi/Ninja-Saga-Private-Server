# Baseline evidence — 2026-11-03

Reference commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd` verified and checked out detached. The malformed reference identifier in the plan header is superseded by `reference/source.json`.

The existing application has **not** been brought up or played. PHP 8.5.11, Composer and Node 26.3.0 are available. MySQL and Docker were not found on PATH; no database schema, seed data or client configuration was changed. Laravel vendor dependencies and omitted game data/assets are not installed/materialized. No user credentials or upstream private keys were reused.

What was executed:

- 388 first-party PHP and 18 realtime JavaScript syntax checks: all pass; see `baseline-static-checks.md`.
- Standalone original SabreAMF serializer and original diagnostic VersionService: generated seven golden binary files (three codec fixtures, four gateway request/response fixtures).
- Rust codec/gateway and configuration tests: 14 passing; reference byte equality covers the diagnostic method, not authentication.

The `.tools/` Rust 1.93.0 GNU installation and portable w64devkit 2.5.0 are workspace-local and ignored by Git. No machine PATH setting was changed. `scripts/dev-env.ps1` activates them for the current shell only.

Next baseline steps: install or provision isolated MySQL, materialize required content, install original locked dependencies in an isolated runtime, inspect seed data before executing destructive seeders, run migrations and representative client flows, record sanitized packet captures. Source-only findings have lower compatibility confidence than captured gameplay.
