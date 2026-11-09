Describe the problem and resulting behavior. Include the release scope and validation evidence.

## Production promotion

- [ ] Required gameplay/client acceptance is complete for the agreed scope.
- [ ] CI, database regression and release build pass on this revision.
- [ ] Runtime artifacts exclude credentials, local tools and experiments.
- [ ] Staging configuration, readiness, assets and database provisioning are verified.
- [ ] Backup, restore and rollback are verified for a deployment change.

Keep the PR draft while applicable gates are incomplete. See `docs/production-readiness.md`. Merging and deployment require separate decisions.
