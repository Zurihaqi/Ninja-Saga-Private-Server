# AMF compatibility spike

NSR-006/009 proof of concept, not a production-complete AMF implementation.
No external dependencies. `cargo test -p amf-codec` runs compatibility and malformed-input tests.

Supports AMF envelopes, headers, multiple bodies, AMF0 null/undefined/boolean/number/string/object/ECMA array/strict array, AMF3 wrappers, signed U29 integers, doubles, strings (including string references), arrays, and anonymous sealed/dynamic objects. It preserves object field order and explicit AMF3 wrappers. Callers explicitly wrap body data even for version-3 envelopes; the envelope version alone does not control value encoding. Gateway normalization and dispatch belong above this codec.

Encoding emits unknown section lengths (`ffffffff`), matching upstream. Response envelopes can use version 0 with marker `11` wrapping AMF3 data, as required by the gateway. Decoder validates finite section lengths, trailing data, UTF-8, depth, message size, string size, and value budgets. These stricter malformed-input rules intentionally differ from the permissive upstream decoder and need gateway error mapping.

Supports acyclic AMF0 object/array references and AMF3 object/array/trait references by expanding them into independent values. Reference identity is not preserved when encoding: the encoder emits inline values and traits. AMF0 reference tables reset at each envelope section; AMF3 tables reset at each AMF3 wrapper, matching the pinned SabreAMF decoder. Active/incomplete object references are rejected as cycles, and invalid indices fail explicitly.

AMF3 ByteArray is supported, including acyclic references and byte/expansion budgets. A PHP serializer fixture covers the binary type used by AIR startup telemetry.

Unsupported: cyclic references, typed/externalizable objects, dates, XML, vectors, dictionaries, combined dense-and-associative AMF3 arrays. Unsupported markers return errors rather than silently corrupting values. The actual AIR client has verified account creation, login and first-mission calls; broader protocol acceptance remains pending. AMF0 integer input encodes as a number; out-of-range AMF3 integers encode as doubles.

## Fixture provenance

Fixtures were generated with the unmodified standalone SabreAMF serializer from pinned commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Regeneration tools remain on the `sprint/1-protocol-compatibility` development branch (`crates/amf-codec/tests/generate-fixtures.php`); PHP and the reference checkout are excluded here. Fixtures are synthetic inputs serialized by the reference codec, **not captured client sessions**. No application services or database are booted by generation.

The three binary fixtures test primitive/associative values and multiple bodies, AMF3 request integer boundaries, and the AMF0-envelope/AMF3-object response shape. Rust tests assert both decoded semantics and byte-identical re-encoding against these fixtures. This does not establish login or full gateway compatibility.

Two additional reference-bearing fixtures are explicitly assembled wire packets whose expected semantics are checked by the pinned PHP deserializer during generation. The upstream serializer always writes inline objects/traits, so these must not be described as serializer-generated packets. They exercise repeated AMF0 objects/arrays and repeated AMF3 objects/arrays/traits. Rust asserts semantic equivalence and semantic round trips; byte identity is not expected after references expand.

Observed reference quirk: `AMF0/Serializer.php::writeArray` tests `isset` on each slot, so a PHP null inside a strict array becomes AMF undefined. The AMF0 fixture deliberately preserves that byte-level observation. The Rust model distinguishes null and undefined rather than globally replacing nulls.

Repeated AMF3 string references charge an aggregate decoded-string budget (the message-byte limit), preventing small packets from expanding into unbounded string allocations. Object/array/value budgets and output byte limits apply independently.

Object-reference expansion checks the entire cached subtree's value count, aggregate string bytes (including keys), and height before allocating its copy. Cached subtrees themselves consume the same value/string budgets, and trait-key copies consume the string budget. Limits are deliberately conservative: a heavily nested or reference-rich input may fit the wire-size limit yet exceed the expanded allocation budget. Tests cover cycles, invalid indices, expanded depth, object/string amplification, truncation, and deterministic malformed mutations.
