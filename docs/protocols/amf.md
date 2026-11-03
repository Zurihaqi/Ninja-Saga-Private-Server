# AMF source contract

Reference commit: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Evidence is static inspection of `app/Http/Controllers/AmfController.php`, `routes/web.php`, and `app/Services/Amf`. No live Flash capture or PHP/Rust differential test has yet established compatibility.

## HTTP and envelope

- `/amf` and `/gateway.php` use `Route::any` and the same controller.
- Empty input returns HTTP 200 text `AMF Gateway Ready`.
- SabreAMF deserialization exceptions return HTTP 500 text `Invalid AMF Data: ` followed by the exception message. Exact malformed-input messages depend on the pinned SabreAMF package.
- Successful serialization returns HTTP 200 `Content-Type: application/x-amf`.
- Every request body produces one response body, preserving order. Its target is the request body's response URI plus `/onResult`; its response field is null.
- AMF3 request encoding causes response envelope encoding AMF0, wrapping every body value with `SabreAMF_AMF3_Wrapper`. Associative PHP arrays recursively become `stdClass` objects; list arrays remain arrays. Empty PHP arrays count as lists. Other encodings retain the incoming encoding.

## Dispatch and arguments

Target is split on `.`. First segment is uppercased at its first character and receives a `Service` suffix unless already present. Second segment is the method, defaulting to `index`; additional segments are ignored. Resolve `App\Services\Amf\{service}` then `App\Services\{service}`. Container construction and `method_exists` precede reflection and `call_user_func_array`. There is no explicit service/method allowlist.

Argument normalization, in this exact order:

1. Non-array body data becomes one argument, even for a zero-parameter method.
2. Array data with a zero-parameter method becomes no arguments.
3. A one-parameter method receives the sole list element if the body is a one-element list; otherwise it receives the full array.
4. A multi-parameter method receives an unwrapped sole nested array if the body is a one-element list containing an array; otherwise the body array is the argument list.

Example: `Character.buyItem` maps to `CharacterService::buyItem($charId, $sessionKey, $itemId, $quantity)`. Both a flat four-element list and a sole nested four-element list are accepted by normalization. This does not prove either passes authentication or business rules.

## Session checks and errors

Gateway precheck excludes exactly `SystemLogin.loginUser`, `SystemLogin.registerUser`, and `SystemLogin.checkVersion` (case-sensitive target comparison). It unwraps a sole nested array, then skips checking non-arrays or arrays without top-level strings of length 32 or 40. It queries users for an exact session-key match against these candidate strings. No match gives `{status: 0, error: "Invalid session key"}`. `SystemLogin.getAllCharacters` bypasses the following character test after matching a user.

All positive numeric top-level arguments are cast to integers and treated as candidate character IDs. No IDs means no ownership check. Any one candidate owned by the matched user passes. Otherwise return `{status: 0, error: "Invalid session key or character ownership mismatch"}`. These are heuristics; they are not a dependable ownership boundary for a Rust service.

Per-service `SessionValidator` is separate: it validates the requested character/user, accepts raw, strict-base64-decoded, SHA-256, or MD5 forms, and hashes **owning user ID** concatenated with the stored key. `SESSION_VALIDATE_BYPASS` disables this validator. Errors contain `status: 0`, `error`, and duplicate `errorMessage`. The gateway can reject a derived 32/40-character key before this validator accepts it. Node uses character ID for its derived key forms instead; see the Socket.IO contract.

Any caught dispatch throwable returns `{status: 500, error: "{target} failed: {exception message}"}` on `/onResult`. Examples:

- Missing class: `Unknown.method failed: Service 'UnknownService' not found for target 'Unknown.method'`.
- Missing method: `Game.missing failed: Method 'missing' not found on service 'GameService' for target 'Game.missing'`.

## Defects and compatibility decisions to resolve

- The invalid-service assertion in `tests/Feature/AmfTest.php` expects `/onStatus` and `level: error`; current controller returns `/onResult` and `status: 500`. The test is stale against this source.
- Gateway logs full request arguments and service results; credentials/session keys can enter logs. Rust must redact them.
- A code comment reserves status 0 for client bans, but session failures still return 0. Preserve observed payloads in fixtures and assess actual client behavior before changing them.
- Generic dispatch potentially exposes utility/development public methods. Inventory membership is not authorization. Build an explicit reviewed registry.
- No explicit controller-level size, depth, or reference limits. Rust codec limits must be documented and tested.

## First compatibility spike

Use `Game.getVersion` (zero arguments, no database) as the first differential fixture: `{version: "1.0.0", server: "Laravel Ninja Saga Revived", status: "active"}`. Test both envelope encodings, multiple bodies, unknown service, unknown method, malformed/truncated messages, null response field, arrays versus objects, reference scope, and argument normalization. Generate expected bytes with the locked PHP SabreAMF implementation, then independently decode and compare with Rust. Handwritten AMF fixtures are useful unit tests but are not captured-client evidence. `SystemLogin.checkVersion` includes random values and deployment URLs and needs explicit normalization rules before comparison.
