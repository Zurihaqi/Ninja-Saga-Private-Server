# Initial architecture contract (NSR-010 provisional)

Three independently runnable Tokio/Axum processes share one Cargo workspace. `shared` owns process config, structured logging, liveness and shutdown. `amf-codec` owns bounded binary values/envelopes without dependencies on HTTP or persistence. `game-server` owns HTTP mapping and explicit dispatch; only Game.getVersion is registered in this spike.

Preserve the existing Flash protocol and MySQL schema during parity work. Add SQLx persistence behind game operations only after schema/transaction contracts are understood. The game server owns durable economy/character writes, PvP temporary match state, chat connection/delivery state. No schema changes or new account semantics were introduced.

Initial interfaces are intentionally narrow: `decode(bytes, Limits) -> Envelope`, `encode(Envelope, Limits) -> bytes`, `shared::run(service, port, Router)`. Explicit `Value::Amf3` represents embedded AMF3 data; the envelope version does not automatically wrap values. Reject unsupported values and bounded malformed input rather than silently coercing it.

AMF production acceptance is withheld: acyclic object/array/trait references now have PHP-decoder-verified fixtures, while real client flows and additional types remain unverified. `socket-protocol` provides bounded legacy text framing; `protocol-spike` exposes an isolated polling echo namespace verified with Socket.IO 2.5.0. Real Flash transport verification still gates selection of the production networking layer. Mass gameplay implementation must not start before these gates.
