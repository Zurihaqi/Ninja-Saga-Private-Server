# AMF compatibility spike

NSR-006/009 proof of concept, not a production-complete AMF implementation.
No external dependencies. `cargo test -p amf-codec` runs compatibility and malformed-input tests.

Supports AMF envelopes, headers, multiple bodies, AMF0 null/undefined/boolean/number/string/object/ECMA array/strict array, AMF3 wrappers, signed U29 integers, doubles, strings (including string references), arrays, and anonymous sealed/dynamic objects. It preserves object field order and explicit AMF3 wrappers. Callers explicitly wrap body data even for version-3 envelopes; the envelope version alone does not control value encoding. Gateway normalization and dispatch belong above this codec.

Encoding emits unknown section lengths (`ffffffff`), matching upstream. Response envelopes can use version 0 with marker `11` wrapping AMF3 data, as required by the gateway. Decoder validates finite section lengths, trailing data, UTF-8, depth, message size, string size, and value budgets. These stricter malformed-input rules intentionally differ from the permissive upstream decoder and need gateway error mapping.

Unsupported: object/trait references and cycles, typed/externalizable objects, dates, XML, byte arrays, vectors, dictionaries, combined dense-and-associative AMF3 arrays. Unsupported markers and reference forms return errors rather than silently corrupting values. Object-reference support and actual Flash captures remain blockers to protocol acceptance. AMF0 integer input encodes as a number; out-of-range AMF3 integers encode as doubles.

## Fixture provenance

`tests/generate-fixtures.php` uses the unmodified standalone SabreAMF serializer from `reference/upstream/Database/Laragon/ninjasage/app/ThirdParty/SabreAMF`, commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Run `php crates/amf-codec/tests/generate-fixtures.php` at workspace root to regenerate. Fixtures are synthetic inputs serialized by the reference codec, **not captured client sessions**. PHP deprecation diagnostics are suppressed for this old library; no application services or database are booted.

The three binary fixtures test primitive/associative values and multiple bodies, AMF3 request integer boundaries, and the AMF0-envelope/AMF3-object response shape. Rust tests assert both decoded semantics and byte-identical re-encoding against these fixtures. This does not establish login or full gateway compatibility.

Observed reference quirk: `AMF0/Serializer.php::writeArray` tests `isset` on each slot, so a PHP null inside a strict array becomes AMF undefined. The AMF0 fixture deliberately preserves that byte-level observation. The Rust model distinguishes null and undefined rather than globally replacing nulls.

Repeated AMF3 string references charge an aggregate decoded-string budget (the message-byte limit), preventing small packets from expanding into unbounded string allocations. Object/array/value budgets and output byte limits apply independently.
