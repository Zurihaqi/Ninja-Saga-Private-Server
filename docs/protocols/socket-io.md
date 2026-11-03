# Socket.IO source contract

Reference commit: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Static source audit only. Dependency versions are read from package locks in the generated [event inventory](../inventory/socket-events.md). Client protocol claims in comments remain unverified until actual client traffic is captured.

## Transport and namespaces

| Setting | Chat | PvP |
|---|---|---|
| Default port | 3002 | 3000 |
| Namespace | `/global-chat`, `/clan-chat` | `/pvp` |
| Transports | websocket, polling | websocket, polling |
| `allowEIO3` | Explicitly true | Not set |
| CORS | wildcard origin; GET/POST | wildcard origin; GET/POST |
| Custom heartbeat settings | None | None |
| HTTP health | `/health`: `{status:"ok", uptime:number}` | same |

The chat comment says the compiled FlashSocket client sends `EIO=3`, requiring compatibility mode. PvP references a Skein client library but does not enable EIO3. Its comment says polling is disabled while its actual transports include polling. Do not copy chat's protocol assumptions into PvP without a capture. Default Socket.IO path, heartbeat timing, upgrade sequence and EIO3 framing must be verified against the locked dependencies/client, not inferred from event names.

## Authentication

Chat receives `auth {character_id, session_key}`; PvP receives `System.auth` with the same fields. Both join `characters` to owning `users`, parse the ID with `parseInt`, trim the provided/stored key, and accept exact raw match, base64-decoded match, SHA-256(character ID + stored key), or MD5(character ID + stored key). Node base64 decoding is permissive. PHP's validator uses owning **user** ID, so derived forms differ whenever user and character IDs differ.

Missing values, invalid numeric parse, absent character, or mismatched key fail authentication. Chat disconnects without an explicit error event. PvP first sends `Notification.disconnect` with the failure text then disconnects. Unexpected errors produce PvP `Server error during authentication`. PvP authenticated commands use a `charId` guard; absence sends `Not authenticated` then disconnects. The wrapper catches synchronous exceptions but does not await returned promises.

Neither Node validator checks session expiration or ban status in these functions. Avoid empty stored-key/derived-key acceptance in the Rust design; upstream accepts empty stored strings through some transformed inputs, so record any security deviation explicitly. Never log keys.

## Chat payload and ordering

`auth` and `open-chatbox` can arrive back-to-back. `_authDone` stores the authentication promise and the open handler awaits it. Clan auth requires a nonempty clan ID and joins `clan_{id}`. Opening sends:

1. `nickname-colors`: character-ID keyed map. Actual `_hslToHex` returns six hex characters **without `#`**, contrary to the file header's example.
2. `online-users`: `{total, users:[{id,name}]}`. Counts sockets, not unique characters. Clan scope uses the current clan room.
3. `history`: array of `{character:{id,name,level,rank,premium}, message}`.

`sendMessage` accepts nonblank strings only, requires authenticated `charId`, silently ignores messages within 3000ms of the last accepted message, trims and slices to 500 JavaScript UTF-16 code units. Accepted messages use the same history record shape and emit `message` to the clan room or global namespace. History retains at most 200 entries in memory; saves are fire-and-forget through `messageStore`. Global history load starts asynchronously in the constructor, so the first opener can receive empty history before it resolves.

`POST /admin/announce` requires nonempty configured `CHAT_ADMIN_SECRET` matching `X-Admin-Secret`. JSON `{channel:"global"|"clan",text:string}` trims text; other channel values route to global. It emits `announcement` string and `message` with a `[System]` character (`id`, level, rank, premium all zero). Clan announcements broadcast namespace-wide and persist only to currently loaded clan histories. Request body has no explicit size bound in the source.

Global emits can reach connected unauthenticated sockets. Clan nickname colors are namespace-wide. Reauthentication on one clan socket does not explicitly leave the old room. These require security decisions; source parity alone is insufficient proof of isolation.

## PvP scope and initial spike

Authentication adds the socket to the lobby (duplicate character connections replace the old connection), sends `Client.characterInfo`, `System.mapIDs`, and broadcasts `System.activePlayers`. Room, matchmaking, spectator and battle events are inventoried with exact call sites. Battle binds additional per-socket handlers after creation, including `Battle.action.*`, `Battle.getPlayerInfo` and battle conversation messages. Full payload schemas and combat semantics are a later audit; the event table is not a claim of protocol implementation.

The compatibility spike must record both services' actual Engine.IO query parameters, opening packets, namespace connects, event frames, heartbeat direction/timing, polling length encoding, WebSocket upgrades, disconnect/reconnect and auth/open-chatbox ordering. Run two original clients against a Rust echo namespace before selecting a library or declaring EIO3 compatibility. Test multibyte text and malformed frames. No Rust Socket.IO implementation has been selected or validated by this source audit.
