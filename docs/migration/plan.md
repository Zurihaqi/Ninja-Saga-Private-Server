# Ninja Saga Private Server — Complete Rust Migration Plan

**Version:** 1.0
**Created:** November 3, 2026
**Status:** November 5 client repair integration — skill/Pet Villa movie repairs, WebSocket realtime, corrected event/social contracts and free former real-money products. Headless regressions pass; full real-client and production acceptance remain open. See [progress](progress.md).
**Project Type:** Full backend rewrite
**Target Language:** Rust
**Development Approach:** Multi-agent engineering
**Source Repository:** https://github.com/zurihaqi/Ninja-Saga-Private-Server

**Reference Commit:** `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`

---

# 1. Executive Summary

## 1.1 Project Overview

This project aims to completely migrate the Ninja Saga Private Server backend from its existing Laravel/PHP and Node.js implementations into Rust.

The migration must preserve compatibility with the existing Adobe AIR/Flash game client.

The resulting server should provide equivalent functionality to the original implementation while offering a maintainable Rust architecture, robust testing, improved observability, and reproducible deployment.

The project will be developed through multiple specialized coding agents working on independent modules under a coordinated integration process.

### Primary objectives

1. Replace the Laravel backend with Rust.
2. Replace the Node.js PvP server with Rust.
3. Replace the Node.js chat server with Rust.
4. Preserve the existing MySQL database structure during initial migration.
5. Preserve compatibility with the existing Flash client.
6. Reimplement the administrative dashboard.
7. Maintain existing gameplay systems and game data.
8. Establish automated testing and compatibility verification.
9. Provide Docker-based deployment.
10. Remove PHP and Node.js runtime dependencies from the final server.

## 1.2 Migration Philosophy

**This is a behavior-preserving rewrite, not a mechanical code translation.**

The existing implementation serves as the reference for:

- Network protocols
- Authentication
- Game mechanics
- Data structures
- Business rules
- Response formatting
- Character progression
- Combat calculations
- Multiplayer interactions
- Persistence behavior

Rust implementations must reproduce verified behavior before introducing optimizations or architectural improvements.

The original repository's claim of being fully functional remains unverified.

A working baseline must be established before treating the existing application as a reliable behavioral reference.

## 1.3 Success Criteria

The project is considered successful when:

- The original game client can connect to the Rust backend.
- Users can register, authenticate, and manage characters.
- Character progression persists correctly.
- Missions and battles function correctly.
- Inventory and equipment systems are operational.
- Skills, talents, pets, and game mechanics behave consistently.
- Multiplayer PvP functions through the existing client.
- World and clan chat function.
- Clan and crew systems are supported.
- Administrative functionality is available.
- Game content and assets remain accessible.
- Database operations are reliable.
- The entire backend can be deployed using Docker Compose.
- No PHP or Node.js server processes are required.
- Required compatibility tests pass.

---

# 2. Existing System Analysis

## 2.1 Repository Information

Source:

https://github.com/zurihaqi/Ninja-Saga-Private-Server

The repository was inspected on November 3, 2026.

### Observed characteristics

| Component | Existing Implementation |
|---|---|
| Main backend | Laravel 12 |
| Backend language | PHP 8.2+ |
| Database | MySQL |
| Admin framework | Filament 4 |
| PvP server | Node.js |
| Chat server | Node.js |
| Realtime communication | Socket.IO |
| Game client | Adobe AIR / Flash; Ruffle feasibility proven, see [ruffle spike](../research/ruffle-client-spike.md) |
| Game protocol | AMF0 / AMF3 |
| Frontend tooling | Vite |
| Database migrations | 34 migration files |
| AMF directory | 73 entries, including subdirectories |
| Admin resources | 15 resource directories |

The repository contains only two commits, both from March 6, 2026.

The project has not been independently verified through a successful installation and gameplay session.

## 2.2 Existing Architecture

The system consists of three major backend services.

### Laravel Game Server

Responsible for:

- Authentication
- Session management
- Character management
- Inventory
- Missions
- Game progression
- Skill and pet management
- Clan and crew systems
- Events
- Administrative operations
- Database persistence
- AMF communication
- Static game asset delivery

### Node.js PvP Server

Responsible for:

- Player authentication
- PvP lobbies
- Battle rooms
- Matchmaking
- Battle state
- Turn management
- Combat calculations
- Spectators
- Multiplayer notifications
- Trophy calculations
- Battle results

### Node.js Chat Server

Responsible for:

- World chat
- Clan chat
- Message broadcasting
- Chat history
- Socket authentication
- Administrative announcements
- Flash socket policy handling

## 2.3 Important Compatibility Constraints

### AMF Protocol

The existing client communicates with the main server using Action Message Format.

The Laravel implementation uses SabreAMF.

Its gateway supports:

- AMF request deserialization
- Multiple request bodies
- Dynamic service dispatch
- AMF0 and AMF3 handling
- Session validation
- Error payload generation
- AMF response serialization

An important behavior is that AMF3 requests are returned using an AMF0 envelope containing AMF3-wrapped response data.

The Rust implementation must preserve this behavior.

The relevant source is:

`Database/Laragon/ninjasage/app/Http/Controllers/AmfController.php`

### Socket.IO Compatibility

The Flash client relies on legacy Socket.IO-compatible networking libraries.

The existing chat server explicitly enables:

`allowEIO3: true`

This is required because its client uses an older Engine.IO handshake.

A modern Rust Socket.IO implementation cannot be assumed compatible without testing.

### Flash Socket Policy

The existing chat server also serves a Flash socket policy on TCP port 843.

This behavior may be required before the client permits certain socket connections.

The replacement must support the necessary policy handshake while restricting access appropriately.

### Static Assets

The Flash client loads SWF assets through HTTP.

Examples include:

- Skills
- Enemies
- Game panels
- Other external resources

The Rust server must preserve required asset paths, response headers, and fallback behavior.

---

# 3. Target Architecture

## 3.1 Architectural Decision

The target architecture will use a **Rust modular monolith with three independently deployable server binaries**.

These binaries will share a Cargo workspace and reusable libraries.

### Game Server

Technology:

- Rust
- Tokio
- Axum
- SQLx
- MySQL

Responsibilities:

- AMF gateway
- Authentication
- Game logic
- Persistence
- REST endpoints
- Administrative APIs
- Static game assets

### PvP Server

Technology:

- Rust
- Tokio
- Compatible Socket.IO implementation or custom compatibility layer

Responsibilities:

- Lobbies
- Rooms
- Matchmaking
- Battle management
- Combat events
- Spectators
- PvP persistence

### Chat Server

Technology:

- Rust
- Tokio
- Compatible Socket.IO implementation or custom compatibility layer

Responsibilities:

- World chat
- Clan chat
- Message storage
- Broadcasting
- Administrative announcements
- Flash socket policies

## 3.2 Proposed Project Structure

```text
ninja-saga-rust/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
│
├── apps/
│   ├── game-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   ├── pvp-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │
│   └── chat-server/
│       ├── Cargo.toml
│       └── src/
│
├── crates/
│   ├── amf-codec/
│   ├── socket-protocol/
│   ├── authentication/
│   ├── game-domain/
│   ├── game-logic/
│   ├── game-data/
│   ├── persistence/
│   └── shared/
│
├── admin/
│   └── web/
│       ├── src/
│       └── package.json
│
├── database/
│   ├── migrations/
│   ├── seeds/
│   └── fixtures/
│
├── tests/
│   ├── compatibility/
│   ├── integration/
│   ├── multiplayer/
│   └── end-to-end/
│
├── docs/
│   ├── architecture/
│   ├── protocols/
│   ├── inventory/
│   ├── decisions/
│   └── agent-tasks/
│
├── reference/
│   └── README.md
│
└── infra/
    ├── docker/
    └── compose.yaml
```

## 3.3 Technology Stack

| Purpose | Technology |
|---|---|
| Language | Rust |
| Runtime | Tokio |
| HTTP framework | Axum |
| Middleware | Tower |
| Database | MySQL |
| Database library | SQLx |
| Internal serialization | Serde |
| AMF protocol | Compatible Rust codec or custom implementation |
| Socket.IO | Compatible Rust implementation or custom protocol layer |
| Logging | tracing |
| Admin frontend | React + TypeScript |
| Containerization | Docker |
| Local orchestration | Docker Compose |
| Testing | Cargo test + integration harness |
| CI | GitHub Actions |

Exact library versions must be pinned and verified during implementation.

## 3.4 Architectural Principles

1. Separate transport protocols from game logic.
2. Keep domain logic independent of HTTP and Socket.IO.
3. Centralize reusable game rules.
4. Avoid unnecessary microservices.
5. Preserve database compatibility initially.
6. Use explicit interfaces between modules.
7. Prefer compile-time validation.
8. Use database transactions for critical state changes.
9. Avoid global mutable state where possible.
10. Support deterministic testing.
11. Treat client-provided actions as untrusted.
12. Keep deployment reproducible.

## 3.5 Database Ownership

The game server is responsible for durable character and economy operations.

PvP owns temporary battle and room state.

Chat owns connection state and message delivery.

All components may use shared persistence libraries, but concurrent writes must follow explicitly defined ownership rules.

Critical operations such as purchasing items, granting rewards, and finishing battles must use transactions and protection against duplicate execution.

---

# 4. Multi-Agent Development Strategy

## 4.1 Team Structure

The project will use six specialized development workstreams and one integration lead.

### Integration Lead

Responsibilities:

- Overall architecture
- Task decomposition
- Interface contracts
- Pull request reviews
- Conflict resolution
- Integration testing
- Milestone coordination
- Release approval

### Agent A — Protocol Engineering

Responsibilities:

- AMF codec
- AMF gateway
- Protocol compatibility
- Service dispatch
- Socket.IO protocol investigation
- Binary fixture testing

Primary ownership:

- `crates/amf-codec`
- `crates/socket-protocol`

### Agent B — Core Game Backend

Responsibilities:

- Authentication
- Sessions
- Character management
- Inventory
- Persistence
- Shared account operations
- Database transactions

Primary ownership:

- `crates/authentication`
- `crates/persistence`
- Core game-server modules

### Agent C — Gameplay Systems

Responsibilities:

- PvE combat
- Missions
- Skills
- Talents
- Pets
- Rewards
- Game progression
- Events
- Economy mechanics

Primary ownership:

- `crates/game-logic`
- `crates/game-data`
- Gameplay services

### Agent D — Multiplayer

Responsibilities:

- PvP server
- Chat server
- Matchmaking
- Battle state
- Multiplayer events
- Reconnection
- Spectator functionality

Primary ownership:

- `apps/pvp-server`
- `apps/chat-server`

### Agent E — Administration

Responsibilities:

- Rust admin APIs
- React dashboard
- User management
- Character management
- Game data management
- Administrative commands
- Permissions

Primary ownership:

- `admin/web`
- Admin-specific game-server modules

### Agent F — Quality and Infrastructure

Responsibilities:

- Docker
- CI/CD
- Automated testing
- Test fixtures
- Security review
- Observability
- Performance testing
- Backup and recovery

Primary ownership:

- `infra`
- `tests`
- CI configuration

## 4.2 Agent Collaboration Rules

All agents must follow these rules:

1. Every task has a designated owner.
2. Every task includes acceptance criteria.
3. Every implementation references the original source.
4. Agents must not modify another workstream's files without coordination.
5. Shared interface changes require integration approval.
6. Every completed feature requires tests.
7. No gameplay changes are allowed during compatibility migration.
8. Protocol differences must be documented.
9. Agents must report unresolved assumptions.
10. Code must compile before submitting a pull request.
11. Agents must not silently modify the existing database schema.
12. The integration lead controls final merges.

## 4.3 Branch Strategy

Use the following naming convention:

`agent/<workstream>/<task-id>`

Examples:

- `agent/protocol/NSR-006`
- `agent/core/NSR-012`
- `agent/multiplayer/NSR-025`

Pull requests should remain narrowly scoped.

## 4.4 Definition of Done for Agent Tasks

A task is complete when:

- Its implementation is finished.
- It compiles successfully.
- Formatting passes.
- Linting passes.
- Unit tests pass.
- Integration tests pass where applicable.
- Compatibility fixtures pass.
- Error behavior is verified.
- Documentation is updated.
- Changes are reviewed.
- No undocumented behavioral differences remain.

---

# 5. Phase 0 — Reverse Engineering and Baseline Establishment

**Priority:** Critical
**Primary Agents:** Integration Lead, Protocol, QA

## Objective

Establish a working understanding of the original system before rewriting it.

## Tasks

### Repository Investigation

- [ ] Pin the original repository commit.
- [ ] Inventory all Laravel services.
- [ ] Inventory all AMF methods.
- [ ] Inventory all HTTP routes.
- [ ] Inventory database models.
- [ ] Inventory database migrations.
- [ ] Inventory seeders.
- [ ] Inventory background jobs.
- [ ] Inventory admin functionality.
- [ ] Inventory static game assets.

### Original Environment

- [ ] Establish isolated development environment.
- [ ] Configure PHP and Composer.
- [ ] Configure MySQL.
- [ ] Configure Laravel.
- [ ] Run migrations.
- [ ] Run seeders.
- [ ] Start PvP server.
- [ ] Start chat server.
- [ ] Configure Flash client.
- [ ] Verify local connectivity.

### Gameplay Baseline

- [ ] Test account registration.
- [ ] Test login.
- [ ] Test character creation.
- [ ] Test character loading.
- [ ] Test inventory.
- [ ] Test missions.
- [ ] Test skill execution.
- [ ] Test rewards.
- [ ] Test PvP.
- [ ] Test chat.
- [ ] Test logout and reconnect.

### Protocol Investigation

- [ ] Capture AMF request samples.
- [ ] Capture AMF response samples.
- [ ] Identify service dispatch conventions.
- [ ] Identify session validation behavior.
- [ ] Identify AMF error responses.
- [ ] Capture Socket.IO handshakes.
- [ ] Capture multiplayer event payloads.
- [ ] Document protocol versions.

### Deliverables

```text
docs/inventory/amf-methods.md
docs/inventory/http-routes.md
docs/inventory/socket-events.md
docs/inventory/database-schema.md
docs/inventory/gameplay-systems.md
docs/inventory/admin-features.md
docs/inventory/background-jobs.md
docs/baseline.md
docs/source-defects.md
```

### Completion Gate

Phase 0 is complete when:

1. The original server can execute a representative end-to-end gameplay flow.
2. Required protocol behavior has been documented.
3. Database relationships are understood.
4. The implementation inventory is sufficiently complete.
5. Unknowns and upstream defects have been explicitly recorded.

If the original server cannot be run, the migration must continue with a documented reduction in compatibility confidence rather than treating untested behavior as established fact.

---

# 6. Phase 1 — Rust Foundation

**Priority:** Critical
**Primary Agents:** Protocol, Core, Infrastructure

## Objective

Establish the Rust architecture and verify that the legacy Flash client can communicate with Rust.

## Tasks

### Workspace

- [ ] Initialize Cargo workspace.
- [ ] Configure Rust toolchain.
- [ ] Configure formatting.
- [ ] Configure Clippy.
- [ ] Configure testing.
- [ ] Configure CI.
- [ ] Define workspace dependencies.

### Core Infrastructure

- [ ] Implement configuration loader.
- [ ] Implement structured logging.
- [ ] Implement error handling.
- [ ] Implement health endpoints.
- [ ] Implement database connection pooling.
- [ ] Implement transaction utilities.
- [ ] Define shared domain types.

### AMF Codec

- [ ] Decode AMF envelopes.
- [ ] Encode AMF envelopes.
- [ ] Support required AMF0 types.
- [ ] Support required AMF3 types.
- [ ] Support object references.
- [ ] Support arrays.
- [ ] Support multiple request bodies.
- [ ] Support response target formatting.
- [ ] Support original error structures.
- [ ] Support the original AMF3 wrapper behavior.
- [ ] Validate malformed payload handling.
- [ ] Implement fixture tests.

### AMF Gateway

- [ ] Implement `/amf`.
- [ ] Implement `/gateway.php`.
- [ ] Implement service registry.
- [ ] Implement method dispatch.
- [ ] Implement session validation middleware.
- [ ] Preserve response content types.
- [ ] Preserve client-visible error behavior.

### Socket.IO Feasibility

- [ ] Identify Flash Engine.IO protocol requirements.
- [ ] Evaluate available Rust implementations.
- [ ] Test namespace connections.
- [ ] Test event framing.
- [ ] Test heartbeats.
- [ ] Test transport compatibility.
- [ ] Test reconnect behavior.
- [ ] Choose library or custom compatibility implementation.

### Deliverables

- Working Rust workspace.
- Working MySQL connection.
- AMF codec proof of concept.
- AMF gateway proof of concept.
- Socket.IO compatibility proof of concept.
- CI pipeline.
- Initial Docker environment.

### Completion Gate

The existing Flash client must successfully communicate with a Rust AMF endpoint.

The project must also demonstrate a viable Rust implementation path for the Flash client's legacy Socket.IO communication.

**Do not begin mass gameplay migration until both protocol risks have been addressed.**

---

# 7. Phase 2 — Core Game Backend

**Priority:** Critical
**Primary Agents:** Core, Gameplay

## Objective

Make the game playable through the Rust server.

## Implementation Order

### Authentication

- [ ] Version validation.
- [ ] Registration.
- [ ] Login.
- [ ] Logout.
- [ ] Session creation.
- [ ] Session validation.
- [ ] Session expiration.
- [ ] Invalid session handling.

### Character Management

- [ ] Character creation.
- [ ] Character selection.
- [ ] Character loading.
- [ ] Character saving.
- [ ] Character attributes.
- [ ] Character levels.
- [ ] Character progression.
- [ ] Equipment loading.

### Inventory

- [ ] Item retrieval.
- [ ] Item ownership.
- [ ] Equipment management.
- [ ] Item purchasing.
- [ ] Item selling.
- [ ] Currency validation.
- [ ] Inventory persistence.

### Skills

- [ ] Skill definitions.
- [ ] Skill ownership.
- [ ] Skill equipment.
- [ ] Skill costs.
- [ ] Skill metadata.
- [ ] Skill SWF delivery.

### Basic PvE

- [ ] Mission loading.
- [ ] Mission entry.
- [ ] Basic combat.
- [ ] Mission completion.
- [ ] Reward calculation.
- [ ] Progression persistence.

## Testing Requirements

Test:

- Correct login.
- Incorrect password.
- Expired session.
- Missing character.
- Character creation.
- Save and reload.
- Equipment persistence.
- Insufficient currency.
- Concurrent purchases.
- Mission rewards.
- Repeated requests.

## Completion Gate

A player must be able to:

1. Launch the existing Flash client.
2. Authenticate against Rust.
3. Create or load a character.
4. Equip items.
5. Enter a mission.
6. Complete the mission.
7. Receive rewards.
8. Log out.
9. Log back in with progression intact.

This is the **First Playable Rust Milestone**.

---

# 8. Phase 3 — Gameplay Systems Migration

**Priority:** High
**Primary Agent:** Gameplay

## Objective

Achieve functional parity with the original gameplay backend.

## 8.1 Combat Systems

- [ ] Damage calculations.
- [ ] Defense calculations.
- [ ] Critical hit calculations.
- [ ] Dodge calculations.
- [ ] Chakra costs.
- [ ] Status effects.
- [ ] Skill cooldowns.
- [ ] Weapon attacks.
- [ ] Enemy attacks.
- [ ] Turn management.
- [ ] Combat rewards.
- [ ] Battle completion.

## 8.2 Character Progression

- [ ] Experience calculation.
- [ ] Level progression.
- [ ] Attribute allocation.
- [ ] Rank progression.
- [ ] Prestige systems.
- [ ] Character restrictions.
- [ ] Progression rewards.

## 8.3 Exams

- [ ] Chunin Exam.
- [ ] Jounin Exam.
- [ ] Special Jounin Exam.
- [ ] Ninja Tutor Exam.
- [ ] Required progression conditions.
- [ ] Exam rewards.

## 8.4 Skills and Talents

- [ ] Skill unlocking.
- [ ] Skill upgrades.
- [ ] Skill effects.
- [ ] Talent acquisition.
- [ ] Talent upgrades.
- [ ] Senjutsu systems.
- [ ] Skill restrictions.
- [ ] Chakra consumption.

## 8.5 Pets

- [ ] Pet ownership.
- [ ] Pet equipment.
- [ ] Pet attributes.
- [ ] Pet skills.
- [ ] Pet progression.
- [ ] Pet combinations.
- [ ] Pet villa systems.

## 8.6 Economy

- [ ] Gold.
- [ ] Tokens.
- [ ] Item purchasing.
- [ ] Item selling.
- [ ] Crafting.
- [ ] Blacksmith.
- [ ] Material markets.
- [ ] Special deals.
- [ ] Reward distribution.

## 8.7 Social Features

- [ ] Friends.
- [ ] Mail.
- [ ] Leaderboards.
- [ ] Player relationships.
- [ ] Clan membership.
- [ ] Crew membership.

## 8.8 Events

Port all implemented event services identified during Phase 0.

Examples observed in the repository include:

- Christmas events.
- Halloween events.
- Dragon Hunt.
- Daily rewards.
- Daily roulette.
- Daily scratch.
- Seasonal and anniversary events.
- Giveaways.

Each event requires individual compatibility and persistence testing.

## Completion Gate

All inventoried gameplay operations must be:

- Implemented and tested;
- Explicitly identified as unused; or
- Explicitly waived with documented justification.

Major gameplay paths must pass compatibility tests.

---

# 9. Phase 4 — PvP Server Migration

**Priority:** High
**Primary Agent:** Multiplayer

## Objective

Replace the existing Node.js PvP server with Rust.

## 9.1 Connection Management

- [ ] Socket.IO connection.
- [ ] Namespace handling.
- [ ] Authentication.
- [ ] Session validation.
- [ ] Character ownership verification.
- [ ] Disconnect handling.
- [ ] Reconnection handling.

## 9.2 Lobby

- [ ] Player presence.
- [ ] Active player counts.
- [ ] Player information.
- [ ] Lobby messages.
- [ ] Available matches.
- [ ] Match notifications.

## 9.3 Rooms

- [ ] Create room.
- [ ] Join room.
- [ ] Leave room.
- [ ] Kick player.
- [ ] Ready state.
- [ ] Skill selection.
- [ ] Room countdown.
- [ ] Room cleanup.

## 9.4 Matchmaking

- [ ] Ranked matchmaking.
- [ ] Casual matchmaking.
- [ ] Queue management.
- [ ] Queue cancellation.
- [ ] Opponent assignment.
- [ ] Match creation.
- [ ] Queue timeout behavior.

## 9.5 Battle Engine

- [ ] Battle initialization.
- [ ] Turn ordering.
- [ ] Battle timers.
- [ ] Weapon attacks.
- [ ] Skills.
- [ ] Dodge.
- [ ] Chakra charging.
- [ ] Scroll usage.
- [ ] Cooldowns.
- [ ] Status effects.
- [ ] Player defeat.
- [ ] Forfeiting.
- [ ] Battle completion.

## 9.6 Spectators

- [ ] Spectator connections.
- [ ] Join battle as spectator.
- [ ] Leave spectator mode.
- [ ] Receive battle events.
- [ ] Spectator count.
- [ ] Permission validation.

## 9.7 Results

- [ ] Winner determination.
- [ ] Trophy calculation.
- [ ] Experience rewards.
- [ ] Currency rewards.
- [ ] Battle history.
- [ ] Persistence.
- [ ] Duplicate reward prevention.

## Concurrency Requirements

PvP should use authoritative server-side battle state.

Recommended design:

- One state machine per active battle.
- Explicit event validation.
- Controlled message processing.
- Bounded queues.
- Deterministic battle calculations.
- Atomic result persistence.
- Reliable disconnect cleanup.

Clients must not be trusted to report final battle outcomes.

## Completion Gate

Two actual game clients must be able to:

1. Authenticate.
2. Enter the PvP lobby.
3. Create or join a room.
4. Start a match.
5. Execute combat actions.
6. Complete the battle.
7. Receive correct rewards.
8. Save the result.
9. Reconnect successfully.

---

# 10. Phase 5 — Chat Server Migration

**Priority:** High
**Primary Agent:** Multiplayer

## Objective

Replace the Node.js chat server with Rust while preserving Flash compatibility.

## 10.1 World Chat

- [ ] Socket authentication.
- [ ] World chat namespace.
- [ ] Send messages.
- [ ] Receive messages.
- [ ] Message history.
- [ ] Connection management.

## 10.2 Clan Chat

- [ ] Clan chat namespace.
- [ ] Clan membership validation.
- [ ] Message broadcasting.
- [ ] Message history.
- [ ] Disconnection handling.

## 10.3 Administrative Features

- [ ] Server announcements.
- [ ] Administrative authentication.
- [ ] Announcement broadcasting.
- [ ] Audit logging.

## 10.4 Legacy Compatibility

- [ ] Engine.IO 3 handshake.
- [ ] Required Socket.IO framing.
- [ ] Namespace connection.
- [ ] Heartbeat behavior.
- [ ] Flash socket policy.
- [ ] TCP port 843 handling.
- [ ] HTTP cross-domain policy.
- [ ] Reconnection.

## 10.5 Security

- [ ] Rate limiting.
- [ ] Message length limits.
- [ ] Input validation.
- [ ] Authentication checks.
- [ ] Clan membership checks.
- [ ] Administrative endpoint protection.

## Completion Gate

Two clients must be able to exchange messages through the existing Flash interface.

World chat, clan chat, message history, and administrative announcements must function correctly.

---

# 11. Phase 6 — Administrative Dashboard

**Priority:** Medium–High
**Primary Agent:** Administration

## Objective

Replace Laravel Filament with a Rust-backed administrative interface.

## Architecture

Frontend:

- React
- TypeScript
- Vite
- A suitable component library

Backend:

- Axum
- Rust REST APIs
- SQLx
- MySQL

## Features

### User Management

- [ ] List users.
- [ ] Search users.
- [ ] View user details.
- [ ] Edit permitted account data.
- [ ] Manage account status.

### Character Management

- [ ] List characters.
- [ ] Search characters.
- [ ] View progression.
- [ ] Manage character attributes.
- [ ] Inspect inventory.
- [ ] Perform authorized character actions.

### Game Data

- [ ] Items.
- [ ] Skills.
- [ ] Pets.
- [ ] Missions.
- [ ] Experience tables.
- [ ] Events.
- [ ] Rewards.
- [ ] Markets.

### Social and Multiplayer Administration

- [ ] Clans.
- [ ] Crews.
- [ ] Seasons.
- [ ] Shadow War.
- [ ] Giveaways.
- [ ] Mail.
- [ ] Admin commands.

### Security

- [ ] Admin authentication.
- [ ] Role-based access control.
- [ ] Server-side authorization.
- [ ] Audit logs.
- [ ] Input validation.
- [ ] Protected sensitive operations.

## Completion Gate

Every required administrative operation identified in the original Filament implementation must have an equivalent secured implementation.

The frontend must not be the sole enforcement point for permissions.

---

# 12. Testing Strategy

## 12.1 Unit Testing

Test individual game rules independently.

Examples:

- Damage formulas.
- Defense calculations.
- Critical hit probability.
- Experience calculations.
- Item prices.
- Reward calculations.
- Skill cooldowns.
- Character attributes.

## 12.2 Protocol Testing

AMF tests must cover:

- Primitive values.
- Objects.
- Arrays.
- References.
- Nested structures.
- AMF0.
- AMF3.
- Mixed AMF0/AMF3 envelopes.
- Invalid payloads.
- Error responses.

Socket.IO tests must cover:

- Handshake.
- Authentication.
- Namespace connection.
- Event delivery.
- Heartbeats.
- Disconnection.
- Reconnection.
- Invalid frames.

## 12.3 Differential Testing

Where possible, execute equivalent inputs against:

1. Original PHP/Node.js implementation.
2. New Rust implementation.

Compare:

- Response structures.
- Data types.
- Error behavior.
- Database mutations.
- Gameplay calculations.
- Persistent state.

Byte-for-byte equality is required only where protocol behavior demands it.

Otherwise, semantic equivalence must be demonstrated.

Differences in timestamps and random values may be normalized only through explicitly documented rules.

## 12.4 Integration Testing

Use isolated MySQL instances.

Test:

- Authentication and persistence.
- Inventory transactions.
- Mission rewards.
- Character updates.
- PvP results.
- Clan operations.
- Chat history.
- Concurrent requests.

## 12.5 End-to-End Testing

Use the actual Flash client for final compatibility verification.

Required flows:

- Registration.
- Login.
- Character creation.
- Inventory.
- PvE.
- PvP.
- Chat.
- Clan/crew.
- Admin operations.
- Logout.
- Reconnection.

## 12.6 Reliability Testing

- [ ] Invalid packets.
- [ ] Concurrent connections.
- [ ] Duplicate requests.
- [ ] Client disconnects.
- [ ] Server restarts.
- [ ] Database interruptions.
- [ ] Failed transactions.
- [ ] Backup restoration.
- [ ] Rollback rehearsal.

---

# 13. Security Requirements

Security hardening must occur throughout development, not only before release.

## Required Actions

- [ ] Replace exposed credentials and keys.
- [ ] Generate unique deployment secrets.
- [ ] Disable production debug output.
- [ ] Prevent secrets from appearing in logs.
- [ ] Validate all AMF input.
- [ ] Limit request sizes and nesting.
- [ ] Validate Socket.IO payloads.
- [ ] Enforce session ownership.
- [ ] Protect against duplicated reward transactions.
- [ ] Implement rate limiting.
- [ ] Use restricted database credentials.
- [ ] Protect administrative APIs.
- [ ] Restrict Flash cross-domain access.
- [ ] Review dependency vulnerabilities.
- [ ] Audit gameplay economy exploits.
- [ ] Review licensing and game-asset distribution rights.

The original repository contains a committed PEM private key and development-oriented configuration values.

These must not be reused as trusted production credentials.

---

# 14. Deployment Architecture

## 14.1 Docker Services

The target deployment consists of:

```text
game-server
pvp-server
chat-server
mysql
reverse-proxy
```

The React admin frontend may be served as static files through the proxy or game server.

## 14.2 Deployment Requirements

- [ ] Multi-stage Docker builds.
- [ ] Reproducible Rust toolchain.
- [ ] Environment-based configuration.
- [ ] Secure secrets handling.
- [ ] Persistent database volumes.
- [ ] Database backup strategy.
- [ ] Health checks.
- [ ] Restart policies.
- [ ] Structured logging.
- [ ] Graceful shutdown.
- [ ] HTTP/HTTPS routing.
- [ ] WebSocket routing.
- [ ] Flash socket policy support.

Port 843 requires special consideration on Linux because it is a privileged port.

Prefer a narrowly scoped port-forwarding or capability configuration rather than running the entire chat service as root.

## 14.3 Deployment Targets

### Development

Docker Compose on a local workstation.

### Testing

Isolated server environment with test accounts and database snapshots.

### Production

Linux VPS with HTTPS/WSS, restricted administrative access, backups, and monitoring.

The production environment must preserve any hostnames expected by the Flash client or use a verified client configuration mechanism.

---

# 15. Development Roadmap

| Phase | Description | Estimated Duration |
|---|---|---|
| 0 | Reverse engineering | 1–2 weeks |
| 1 | Rust foundation | 1–2+ weeks |
| 2 | Core playable backend | 2–4 weeks |
| 3 | Gameplay migration | 4–8+ weeks |
| 4 | PvP and chat | 2–4+ weeks |
| 5 | Admin and secondary systems | 3–6 weeks |
| 6 | Hardening and release | 2–4+ weeks |

These durations are preliminary estimates.

Several phases may overlap after shared interfaces have stabilized.

### Milestone Targets

**Milestone 1 — Protocol Compatibility**

Rust can communicate with the Flash client.

**Milestone 2 — First Playable Build**

Rust handles authentication, character loading, and basic gameplay.

**Milestone 3 — Gameplay Parity**

Core PvE and progression systems work.

**Milestone 4 — Multiplayer Parity**

Rust PvP and chat servers function.

**Milestone 5 — Administration Complete**

Required administrative operations are implemented.

**Milestone 6 — Rust-Only Release**

Complete backend operates without PHP or Node.js server processes.

### Overall Estimates

- First playable build: approximately 4–8 weeks.
- Broad gameplay and multiplayer functionality: approximately 2–4 months.
- Complete tested migration: potentially 4–6+ months.

These are estimates, not commitments.

Protocol compatibility and incomplete original behavior may extend the schedule.

---

# 16. Sprint 0 — Initial Agent Backlog

The first sprint focuses on reducing architectural uncertainty.

| Task ID | Owner | Description |
|---|---|---|
| NSR-001 | QA | Establish original server baseline |
| NSR-002 | Protocol | Inventory AMF services and methods |
| NSR-003 | Multiplayer | Audit Socket.IO and Flash networking |
| NSR-004 | Core | Inventory database schema and seeders |
| NSR-005 | Admin | Inventory Filament, HTTP APIs, and jobs |
| NSR-006 | Protocol | Implement AMF codec proof of concept |
| NSR-007 | Multiplayer | Implement legacy Socket.IO compatibility spike |
| NSR-008 | Infrastructure | Initialize Rust workspace and Docker environment |
| NSR-009 | QA | Build compatibility fixture harness |
| NSR-010 | Integration Lead | Approve architecture and freeze initial contracts |

## Execution Order

### Wave 1 — Independent Research

Launch:

- NSR-001
- NSR-002
- NSR-003
- NSR-004
- NSR-005

These tasks can proceed independently.

### Wave 2 — Technical Validation

After receiving the required research outputs:

- NSR-006
- NSR-007
- NSR-008
- NSR-009

### Wave 3 — Integration

Execute NSR-010.

Review:

- Protocol feasibility.
- Database assumptions.
- Rust module boundaries.
- Shared interfaces.
- Technical risks.
- Phase 1 readiness.

Do not begin full gameplay implementation until this review passes.

---

# 17. Risk Register

| Risk | Severity | Mitigation |
|---|---|---|
| Original server cannot run | Critical | Establish baseline before rewrite |
| AMF incompatibility | Critical | Codec proof of concept and fixtures |
| Legacy Socket.IO incompatibility | Critical | Real Flash client handshake testing |
| Gameplay behavior differences | High | Differential testing |
| Database corruption | High | Transactions and backups |
| Reward duplication | High | Idempotency and concurrency testing |
| Agent code conflicts | High | Module ownership and contracts |
| Incomplete game content | High | Feature inventory |
| Security vulnerabilities | High | Continuous security review |
| Admin feature omissions | High | Filament operation inventory |
| Project overengineering | Medium | Modular monolith |
| Public distribution rights | High | Legal and asset review |

---

# 18. Architecture Decision Records

The following decisions should be documented before implementation.

## ADR-001 — Preserve Flash Client Compatibility

The Flash client remains unchanged during backend migration.

## ADR-002 — Rust Workspace Architecture

Use three independently executable Rust servers with shared libraries.

## ADR-003 — Preserve MySQL Schema

Avoid schema redesign during initial parity work.

## ADR-004 — AMF Codec Strategy

Evaluate available Rust implementations and develop missing protocol functionality as required.

## ADR-005 — Legacy Socket.IO Compatibility

Select or implement a Rust protocol layer that demonstrably supports the existing Flash client.

## ADR-006 — Domain and Persistence Ownership

Define service responsibilities and transaction boundaries.

## ADR-007 — Administrative Interface

Replace Filament with React/TypeScript and Rust REST APIs.

## ADR-008 — Migration and Rollback

Maintain a working original implementation until final acceptance.

---

# 19. Agent Task Template

All implementation tasks should follow this structure.

```markdown
# Task ID — Title

## Owner

Assigned agent:

## Priority

Critical / High / Medium / Low

## Source Reference

Original source files and methods.

## Objective

Describe the expected outcome.

## Dependencies

List required interfaces and completed tasks.

## Scope

Describe functionality to implement.

## Out of Scope

Explicitly list excluded functionality.

## Protocol Contract

Document input, output, errors, and side effects.

## Implementation Requirements

List technical constraints.

## Acceptance Criteria

- [ ] Successful behavior verified.
- [ ] Error behavior verified.
- [ ] Boundary conditions tested.
- [ ] Database effects verified.
- [ ] Compatibility tests pass.

## Testing

Describe unit, integration, and compatibility tests.

## Risks

Document unresolved implementation concerns.

## Deliverables

List expected files, documentation, and tests.

## Definition of Done

Specify the evidence needed for approval.
```

---

# 20. Integration and Quality Gates

## Pull Request Requirements

Every pull request must pass:

- Rust compilation.
- `cargo fmt`.
- `cargo clippy`.
- Unit tests.
- Applicable integration tests.
- Protocol compatibility tests where applicable.
- Security and authorization checks.
- Integration review.

## Contract Changes

Changes to shared interfaces require approval.

Examples:

- AMF response models.
- Socket.IO payload structures.
- Character types.
- Battle state models.
- Database schema.
- Authentication behavior.

## Compatibility Policy

Agents must not silently change:

- Gameplay calculations.
- Client-visible error messages.
- Response types.
- Reward values.
- Progression requirements.
- Network event names.
- Existing persistent data formats.

Any intentional deviation must be documented and approved.

---

# 21. Final Release Checklist

## Functionality

- [ ] Authentication works.
- [ ] Characters work.
- [ ] Inventory works.
- [ ] Skills work.
- [ ] PvE works.
- [ ] Progression works.
- [ ] Pets work.
- [ ] Events work.
- [ ] PvP works.
- [ ] Chat works.
- [ ] Clan systems work.
- [ ] Crew systems work.
- [ ] Admin dashboard works.
- [ ] Static assets load correctly.

## Technical

- [ ] No PHP server dependency.
- [ ] No Node.js server dependency.
- [ ] MySQL migrations work.
- [ ] Docker deployment works.
- [ ] Health checks pass.
- [ ] Logging works.
- [ ] Secrets are protected.
- [ ] Backup and restore tested.
- [ ] Security tests pass.
- [ ] Compatibility tests pass.
- [ ] Load tests pass.
- [ ] Rollback procedure documented.

## Documentation

- [ ] Architecture documentation.
- [ ] Database documentation.
- [ ] AMF protocol documentation.
- [ ] Socket.IO protocol documentation.
- [ ] Gameplay service inventory.
- [ ] Administrative API documentation.
- [ ] Installation guide.
- [ ] Deployment guide.
- [ ] Troubleshooting guide.
- [ ] Backup and recovery guide.

---

# 22. Post-Migration Improvements

The following improvements should be considered only after compatibility goals are satisfied.

### Performance

- Optimize frequently executed SQL queries.
- Introduce caching where measured bottlenecks exist.
- Profile combat calculations.
- Reduce unnecessary allocations.
- Improve connection handling.

### Architecture

- Refine module boundaries.
- Improve background job processing.
- Introduce distributed state only when scaling requires it.
- Consider separating individual services only where operationally justified.

### Gameplay

- Additional game content.
- Bug fixes.
- Game balance improvements.
- New events.
- Administrative tooling improvements.

### Client Modernization

Potential long-term research:

- Replace Adobe AIR.
- Reimplement the Flash frontend.
- Investigate HTML5 game rendering.
- Modernize client networking.

These are separate initiatives and must not delay backend parity.

---

# 23. Important Implementation Notes

## Do Not Rewrite Everything Simultaneously

Although the final objective is a complete Rust backend, migration should occur incrementally.

The original implementation should remain usable throughout development.

This allows each Rust module to be verified against known behavior.

## Prefer Observable Behavior Over Assumptions

The original source may contain:

- Bugs.
- Inconsistent conventions.
- Undocumented edge cases.
- Incomplete features.
- Hardcoded values.
- Legacy protocol requirements.

Agents should report these findings rather than silently correcting them.

## Avoid Premature Optimization

Rust does not automatically make a poorly designed system efficient.

Prioritize:

1. Correctness.
2. Compatibility.
3. Data integrity.
4. Maintainability.
5. Performance.

## Maintain a Shared Knowledge Base

Agents should document findings centrally.

Recommended documents:

```text
docs/
├── architecture/
│   ├── overview.md
│   ├── domain-boundaries.md
│   └── decisions/
│
├── protocols/
│   ├── amf.md
│   ├── socket-io.md
│   └── flash-policy.md
│
├── inventory/
│   ├── amf-methods.md
│   ├── http-routes.md
│   ├── socket-events.md
│   ├── database-schema.md
│   └── admin-features.md
│
├── migration/
│   ├── progress.md
│   ├── compatibility-matrix.md
│   ├── blockers.md
│   └── release-checklist.md
│
└── agent-tasks/
    ├── backlog.md
    └── completed/
```

---

# 24. Immediate Next Actions

The project should begin with the following actions:

1. Establish a new repository for the Rust implementation.
2. Pin the original Ninja Saga repository as a reference.
3. Establish an isolated original-server environment.
4. Create the architecture and protocol inventories.
5. Create the Rust workspace.
6. Investigate AMF compatibility.
7. Investigate legacy Socket.IO compatibility.
8. Establish database and integration testing infrastructure.
9. Review initial findings.
10. Approve Phase 1 implementation.

**The first major technical milestone is successfully logging into Ninja Saga through the existing Flash client while its main backend is implemented in Rust.**

Once achieved, the remaining migration can proceed through independently testable feature groups.

---

# 25. Source References

Original repository:

https://github.com/zurihaqi/Ninja-Saga-Private-Server

Pinned reference commit:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/commit/ecf9012d885d1c89fd5f42791eed4f37f90f2edd

Laravel dependencies:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/composer.json

AMF gateway:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/app/Http/Controllers/AmfController.php

Laravel routes:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/routes/web.php

AMF services:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/tree/main/Database/Laragon/ninjasage/app/Services/Amf

Database migrations:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/tree/main/Database/Laragon/ninjasage/database/migrations

PvP documentation:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/pvp-server/README.md

PvP implementation:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/pvp-server/index.js

Chat implementation:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/chat-server/index.js

Environment configuration:

https://github.com/zurihaqi/Ninja-Saga-Private-Server/blob/main/Database/Laragon/ninjasage/.env.example

---

**End of Migration Plan — Version 1.0**

**Current milestone: complete gameplay loops on the existing branch. See [current progress and remaining work](progress.md); the sprint sequence above is retained as the original migration reference.**
