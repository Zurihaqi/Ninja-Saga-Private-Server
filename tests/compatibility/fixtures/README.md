# Gateway golden fixtures

Regenerate on the `sprint/1-protocol-compatibility` development branch from its repository root. The PHP generator and reference checkout are intentionally excluded from runtime review:

```sh
php tests/compatibility/generate-gateway-fixtures.php
```

These four files are **source-derived generated HTTP bodies**, not packet captures from a running Laravel server or Flash client. Generation requires the pinned reference checkout at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`, PHP 8.2+, and no Composer/Laravel boot or database. `manifest.json` records source and fixture SHA-256 hashes. The generator was exercised with PHP 8.5.11.

The generator uses the upstream vendored SabreAMF classes and calls the actual upstream `App\Services\Amf\GameService\VersionService::getVersion()`. It mirrors the controller's AMF3 array/object normalization, envelope encoding selection, and response-target construction. It does not execute routing, middleware, session checks, service lookup, or the Laravel controller itself; those remain separate compatibility work.

| File | Expected meaning |
|---|---|
| `game-version-amf0-request.amf` | AMF0 request, target `Game.getVersion`, response `/1`, empty argument array |
| `game-version-amf0-response.amf` | AMF0 response, target `/1/onResult`, empty response URI, associative array result |
| `game-version-amf3-request.amf` | AMF3 request, same target/response/arguments |
| `game-version-amf3-response.amf` | AMF0 envelope with AMF3-wrapped object result, target `/1/onResult`, empty response URI |

The version result is `{version: "1.0.0", server: "Laravel Ninja Saga Revived", status: "active"}` in that insertion order. The controller's null response URI serializes as an empty string. All body length fields use the upstream serializer's unknown-length sentinel. HTTP tests should POST each request to both `/amf` and `/gateway.php`, assert 200 and `application/x-amf`, and compare the response bytes to its matching golden fixture.

Byte equality here verifies one deterministic source-derived operation and wrapper shape. It does not establish login, full AMF coverage, or actual Flash client compatibility.
