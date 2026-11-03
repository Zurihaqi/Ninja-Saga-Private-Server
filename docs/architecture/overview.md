# Initial architecture contract (NSR-010 provisional)

Three independently runnable Tokio/Axum processes share one Cargo workspace. `shared` owns process config, structured logging, liveness and shutdown. `amf-codec` owns bounded binary values/envelopes without dependencies on HTTP or persistence. `game-server` owns HTTP mapping and explicit dispatch; only Game.getVersion is registered in this spike.

Preserve the existing Flash protocol and MySQL schema during parity work. Add SQLx persistence behind game operations only after schema/transaction contracts are understood. The game server owns durable economy/character writes, PvP temporary match state, chat connection/delivery state. No schema changes or new account semantics were introduced.

Initial interfaces are intentionally narrow: `decode(bytes, Limits) -> Envelope`, `encode(Envelope, Limits) -> bytes`, `shared::run(service, port, Router)`. Explicit `Value::Amf3` represents embedded AMF3 data; the envelope version does not automatically wrap values. Reject unsupported values and bounded malformed input rather than silently coercing it.

AMF production acceptance is withheld: object/trait references and real client flows remain unverified. Socket.IO library selection is deferred until an Engine.IO 3 transport spike and real client handshake succeed. Mass gameplay implementation must not start before these gates.
