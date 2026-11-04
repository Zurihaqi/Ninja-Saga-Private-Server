# Runtime progress — 2026-11-04

Status: **gameplay integration and client acceptance in progress**. [Draft PR #1](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/pull/1) incorporates development checkpoint `68dbf2d7`. Original `main` remains preserved on `legacy/php-node` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. The PR has not been merged or deployed.

| Area | Current evidence |
|---|---|
| Core PvE/progression | Accounts, equipment, points, shops, academy/presets, missions, four exams, talents, Sage Mode and crafting have headless persistence checks; first mission verified in AIR and reported in Ruffle desktop |
| Pets/inventory | Villa/combining, feeding, paid skill learning, animation buying/use, consumables and account tools implemented and checked headlessly |
| PvE/economy modes | Hunting, Eudemon, daily rewards, markets and Shadow War implemented; representative MySQL flows pass |
| Social | Friends, recruitment and mail implemented; clan/crew HTTP APIs exist, while the AMF bridge still has reference-style placeholder behavior requiring client integration |
| Events/admin | Nine seasonal handlers, packages/giveaways/leaderboards, administrative REST API and React dashboard implemented; representative checks, not exhaustive event/client acceptance |
| Realtime | Chat and PvP now have authenticated polling implementations and lifecycle/router tests. Actual two-client acceptance and required WebSocket support remain open |
| Browser/runtime | React/Ruffle host and admin frontend build; Ruffle assets/licenses generated from the locked dependency. Runtime image definitions package all three servers, import/migration utilities and built web assets |

This integration closes missing-session clan/crew token issuance, saves token rotations atomically, removes built-in admin/chat secrets and requires bcrypt-backed admin passwords. Local admin access uses the explicitly configured secret or an admin account. New pet learning has an additive schema migration; existing pet unlock behavior is retained.

Development validation passes 32 regular Rust tests, strict Clippy, formatting, minimal-fixture and original-catalog MySQL regressions, and the browser build. Live checks verified the game/browser host, authenticated admin overview and chat/PvP liveness. Runtime-only verification and hosted CI are recorded in the PR; the four excluded protocol-experiment tests account for the lower runtime test count.

## Remaining work

1. Complete real-client progression and natural TP/SS/material earning; finish staged S missions and remaining actions found through that audit. `getMissionSData` currently provides the reference's initial data contract.
2. Connect and accept clan/crew panels against the HTTP services; finish the AMF behavior actually required by the client.
3. Accept complete chat/PvP flows with two clients, including rewards, spectators and reconnects; confirm polling suffices or add the required WebSocket support.
4. Accept later exams, pets, events and administration in AIR/Ruffle/browser. A frontend build and headless tests do not establish full panel compatibility.
5. Verify containers, migrations, asset delivery, backups/restore and rollback in staging. Readiness still returns 503; battle sessions remain in memory and clear on restart.

See [production acceptance](production-readiness.md). No full-parity claim or completion percentage is made. Development helpers, original assets, credentials, protocol experiments and migration task notes remain excluded from this branch.
