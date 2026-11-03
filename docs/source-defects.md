# Source defects and compatibility risks

All findings refer to the pinned reference; syntax checks do not establish runtime effects. Upstream files remain unchanged.

| Finding | Evidence / consequence |
|---|---|
| Stale invalid-service test | `tests/Feature/AmfTest.php` expects `/onStatus` and `level:error`; controller emits `/onResult`, numeric status 500. Follow executable controller behavior in future differential tests. |
| Session heuristics | Gateway bypasses global validation when no top-level 32/40-character string exists. Numeric arguments are treated as candidate character IDs. Replace only under an explicit documented security/compatibility decision. |
| Derived-key mismatch | PHP SessionValidator derives from user ID; Node chat/PvP derive from character ID. Requires actual login and socket captures. |
| Sensitive logging | Gateway logs arguments and results wholesale. Rust logs process metadata only. |
| Legacy transport mismatch | Chat enables `allowEIO3`; PvP does not. Both configure polling and websocket. A modern Socket.IO crate alone is not evidence of Flash compatibility. |
| Clan migration collision | Two migrations create overlapping clan tables with differing constraints/defaults and shared rollback ownership. Do not blindly translate migration history. |
| Chat schema outside Laravel | `chat-server/src/messageStore.js` creates chat_messages; migration inventory alone is insufficient. |
| Destructive seed fallback | GameDataSeeder deletes data before loading files that may be absent; missing files can become empty arrays. Inspect data before seeding. |
| Admin email privilege | User::canAccessPanel permits a specific email address as well as type 99; verify registration/authorization consequences before porting. |
| Crew aliases / CSRF | Root aliases lack the same CSRF exemptions as prefixed routes. Runtime requests must establish actual behavior. |
| AMF null array slot | SabreAMF AMF0 strict-array serialization emits undefined for PHP null slots due to `isset`. Golden fixture records this. |

Detailed source links: `inventory/`, `protocols/amf.md`, `protocols/socket-io.md`.
