# Branches and production promotion

`main` is reserved for reviewed production-ready changes. The Rust migration is not production-ready yet; its runtime PR remains draft until the acceptance items below are satisfied. Creating a PR does not authorize a merge or deployment.

| Branch | Purpose |
|---|---|
| `legacy/php-node` | Preserved original PHP/Node implementation, original client and assets; created from the existing `main` before the proposed Rust replacement. |
| `sprint/1-protocol-compatibility` | Continuing Rust development, local setup tools, migration inventories, experiments and headless test tooling. Reuse this branch across sprints. |
| `release/rust-runtime` | Review branch containing runtime source, required catalogs/schema, regression tests, CI, operational documentation and packaging. Local workstation tools and the isolated protocol experiment are excluded. |
| `main` | Production destination. Changes arrive through a reviewed PR after acceptance; no migration cutover has been performed. |

The legacy archive preserves commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Keep it available while migration parity and operational rollback are unresolved. A branch archive preserves source history; it is not a database backup.

## Required before promoting the Rust runtime

- Complete the agreed gameplay scope, including progression reward sources and required social/realtime systems, or explicitly agree a narrower release scope.
- Verify the supported progression flows in the actual AIR client beyond the first mission; headless success alone does not establish client acceptance.
- Pass CI, MySQL regressions, strict Clippy, formatting and the release build for the exact reviewed revision.
- Validate container startup, database/catalog provisioning, asset delivery and meaningful readiness checks in the target environment.
- Validate configuration, credentials, backups, restore and rollback procedures using staging data.
- Review and approve the PR. Merge and deployment are separate actions.

Keep tests and CI in the runtime review branch: they provide evidence for release acceptance. Exclude workstation credentials, `.tools/`, downloaded reference/client assets, logs, build outputs, migration task notes and protocol experiments. Production images contain only the required executable and runtime files.

This document states the release policy. GitHub branch-protection/ruleset settings must be verified separately; their enforcement is not implied by this file.
