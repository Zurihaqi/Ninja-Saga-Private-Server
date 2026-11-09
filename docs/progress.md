# Runtime progress — 2026-11-05

Status: **client repair integration and acceptance in progress**. This runtime candidate incorporates development checkpoints `f77fc185` and `e712103e`. [PR #1](https://github.com/Zurihaqi/Ninja-Saga-Private-Server/pull/1) remains draft; main is not merged or deployed. Original source/assets remain preserved on `legacy/php-node` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`.

## This repair batch

- Skill inventory shows all owned skills, isolates/cancels stale loaders and handles missing artwork without freezing selection/removal. Presets follow client prices: first free, then 200 premium/600 free-account tokens, maximum four.

- Pet Villa clears reused model holders, cancels stale requests and safely refreshes timers. Static pet previews now load 141 generated silent movies to prevent overlapping sound effects escaping the Villa. Binary checks preserve sound bindings/duration and confirm combat assets remain unchanged. User acceptance after a full game restart is pending. Backend aliases, balances and level caps match its client contract.

- Talent/Sage Upgrade Max accepts Flash numeric flags. Second Secret discovery requires Special Jounin rank 6.

- Chat/PvP support direct Engine.IO 3/4 WebSocket and polling, with authentication/lifecycle tests and actual legacy Socket.IO 2.5 connection checks.

- Clan/crew REST routes and membership payloads match the shipped client. Both crew battle phases persist/replay results; clan quick/manual prestige rewards persist.

- Event fixes address actual argument positions, boss IDs, catalog rewards/formulas, minigame codes/replay, gacha prices/coin balances and milestone quantities. Explicit `seed-events` inserts missing reference settings while preserving existing records and keeping unsupported panels inactive.

- Former real-money products are free through authenticated local claims: Recharge token packs, permanent Emblem/Trial Emblem, Ancestor and four package tiers. Requests deduplicate and preserve administrator roles. The first Emblem activation grants a Wisdom scroll. Ordinary gold/token prices remain.

The prepared local movie was compiled and re-exported to verify all 12 repaired classes. Required preparation scripts ship under `client/`; generated movies/assets and workstation tools remain excluded. Separate in-progress admin redesign edits are not part of this batch.

Event images are now served from the local asset tree, and optional banner loading has a five-second deadline plus guarded task completion. The user confirmed the browser gets past startup to login/the game.

## Evidence and remaining work

The development tree passes 36 regular Rust tests, strict Clippy, frontend build, and minimal/original-catalog isolated MySQL regressions. Live AMF checks verify free products/replays, all package tiers, presets, Villa and Wisdom data using a disposable account. The runtime tree repeats its own checks; hosted results are recorded in the PR. Four development protocol-experiment tests are excluded here.

Complete real-client acceptance is still required. Seven upstream skill movies are missing. Additional seasonal event handlers, staged S missions and natural resource progression remain incomplete; inactive event definitions are not a completion claim. Full PvP rewards/reconnect/spectator flows need two-client acceptance. Containers, meaningful readiness (currently 503), backups/restore and rollback still require staging verification. Battles are held in memory and clear on restart.

See [production acceptance](production-readiness.md). This is not full parity or production approval.
