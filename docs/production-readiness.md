# Production acceptance and branches

`main` is reserved for reviewed, production-ready changes. This Rust replacement remains a **draft PR** because gameplay parity and operational acceptance are unfinished. Local tests alone do not approve a merge or deployment.

| Branch | Purpose |
|---|---|
| `legacy/php-node` | Original PHP/Node implementation and client/assets preserved from `main`. |
| `sprint/1-protocol-compatibility` | Continuing development, workstation tools and protocol experiments. |
| `release/rust-runtime` | Runtime-focused review: source, tests, CI, required data and operating docs. |
| `main` | Production destination after acceptance and review. |

The archive matches original commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Keep it available until parity and rollback are resolved. A source archive is not a database backup.

## Required before promotion

- [ ] Complete required gameplay and progression reward sources, or explicitly agree a narrower release scope.
- [ ] Verify supported progression in the AIR client beyond the first mission.
- [ ] Pass runtime CI, MySQL regression, formatting, strict Clippy and release build on the reviewed revision.
- [ ] Verify container startup, database/catalog provisioning and assets in staging.
- [ ] Implement meaningful readiness; current 503 blocks production acceptance.
- [ ] Verify configuration, credentials, backups, restore and rollback using staging data.
- [ ] Review and approve the PR before merging; decide deployment separately.

Tests and CI remain as release evidence. Workstation credentials, downloaded assets, logs, build outputs, protocol experiments, PHP/Node fixture generators and migration task notes are excluded. The image contains the game executable and explicit import utility.

This is written promotion policy. GitHub branch protection/rulesets have not been configured or verified by this change; enforcement must be checked separately.
